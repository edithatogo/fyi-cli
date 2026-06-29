use arti_client::{TorClient, TorClientConfig};
use std::net::IpAddr;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tor_rtcompat::PreferredRuntime;

#[derive(thiserror::Error, Debug)]
pub enum TorError {
    #[error("Arti client error: {0}")]
    Arti(#[from] arti_client::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tor runtime error: {0}")]
    Runtime(String),

    #[error("SOCKS handshake protocol error: {0}")]
    Protocol(String),
}

#[derive(Debug, Clone)]
pub struct TorBootstrapStatus {
    pub ready: bool,
    pub progress: f32,
}

pub struct TorManager {
    client: TorClient<PreferredRuntime>,
    proxy_addr: Option<SocketAddr>,
}

impl TorManager {
    pub fn new() -> Result<Self, TorError> {
        let config = TorClientConfig::default();
        let runtime = PreferredRuntime::current().map_err(|e| TorError::Runtime(e.to_string()))?;
        let client = TorClient::with_runtime(runtime)
            .config(config)
            .create_unbootstrapped()
            .map_err(TorError::Arti)?;

        Ok(Self {
            client,
            proxy_addr: None,
        })
    }

    pub fn client(&self) -> &TorClient<PreferredRuntime> {
        &self.client
    }

    pub async fn bootstrap(&self) -> Result<(), TorError> {
        self.client.bootstrap().await.map_err(TorError::Arti)
    }

    pub fn get_bootstrap_status(&self) -> TorBootstrapStatus {
        let status = self.client.bootstrap_status();
        TorBootstrapStatus {
            ready: status.ready_for_traffic(),
            progress: status.as_frac(),
        }
    }

    pub async fn start_proxy(&mut self) -> Result<(SocketAddr, oneshot::Sender<()>), TorError> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let local_addr = listener.local_addr()?;
        self.proxy_addr = Some(local_addr);

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let client_clone = self.client.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((stream, _)) => {
                                let c = client_clone.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_socks_connection(stream, c).await {
                                        eprintln!("Tor SOCKS connection handling error: {:?}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                eprintln!("Tor SOCKS accept error: {:?}", e);
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        break;
                    }
                }
            }
        });

        Ok((local_addr, shutdown_tx))
    }

    pub fn create_reqwest_client(&self) -> Result<reqwest::Client, TorError> {
        let proxy_addr = self.proxy_addr.ok_or_else(|| {
            TorError::Runtime("Proxy has not been started yet. Call start_proxy first.".to_string())
        })?;

        let proxy_url = format!("socks5h://{}", proxy_addr);
        let proxy = reqwest::Proxy::all(&proxy_url)
            .map_err(|e| TorError::Runtime(format!("Failed to parse proxy URL: {}", e)))?;

        let client = reqwest::Client::builder()
            .proxy(proxy)
            .build()
            .map_err(|e| TorError::Runtime(format!("Failed to build reqwest Client: {}", e)))?;

        Ok(client)
    }
}

async fn handle_socks_connection(
    mut client_stream: TcpStream,
    tor_client: TorClient<PreferredRuntime>,
) -> Result<(), TorError> {
    let mut header = [0u8; 2];
    client_stream.read_exact(&mut header).await?;
    let version = header[0];
    let nmethods = header[1];

    if version != 5 {
        return Err(TorError::Protocol(format!(
            "Unsupported SOCKS version: {}",
            version
        )));
    }

    let mut methods = vec![0u8; nmethods as usize];
    client_stream.read_exact(&mut methods).await?;

    if !methods.contains(&0x00) {
        client_stream.write_all(&[5, 0xFF]).await?;
        return Err(TorError::Protocol("No acceptable auth methods".to_string()));
    }

    client_stream.write_all(&[5, 0x00]).await?;

    let mut request_header = [0u8; 4];
    client_stream.read_exact(&mut request_header).await?;
    let ver = request_header[0];
    let cmd = request_header[1];
    let atyp = request_header[3];

    if ver != 5 {
        return Err(TorError::Protocol(format!(
            "Unsupported request version: {}",
            ver
        )));
    }
    if cmd != 1 {
        client_stream
            .write_all(&[5, 0x07, 0, 1, 0, 0, 0, 0, 0, 0])
            .await?;
        return Err(TorError::Protocol(format!(
            "Unsupported SOCKS command: {}",
            cmd
        )));
    }

    let dest_host = match atyp {
        1 => {
            let mut ip = [0u8; 4];
            client_stream.read_exact(&mut ip).await?;
            IpAddr::from(ip).to_string()
        }
        3 => {
            let mut len_buf = [0u8; 1];
            client_stream.read_exact(&mut len_buf).await?;
            let len = len_buf[0] as usize;
            let mut domain_buf = vec![0u8; len];
            client_stream.read_exact(&mut domain_buf).await?;
            String::from_utf8(domain_buf)
                .map_err(|_| TorError::Protocol("Invalid UTF-8 in domain name".to_string()))?
        }
        4 => {
            let mut ip = [0u8; 16];
            client_stream.read_exact(&mut ip).await?;
            IpAddr::from(ip).to_string()
        }
        _ => {
            client_stream
                .write_all(&[5, 0x08, 0, 1, 0, 0, 0, 0, 0, 0])
                .await?;
            return Err(TorError::Protocol(format!(
                "Unsupported address type: {}",
                atyp
            )));
        }
    };

    let mut port_buf = [0u8; 2];
    client_stream.read_exact(&mut port_buf).await?;
    let dest_port = u16::from_be_bytes(port_buf);

    let tor_stream = match tor_client.connect((dest_host.as_str(), dest_port)).await {
        Ok(stream) => stream,
        Err(e) => {
            client_stream
                .write_all(&[5, 0x01, 0, 1, 0, 0, 0, 0, 0, 0])
                .await?;
            return Err(TorError::Arti(e));
        }
    };

    client_stream
        .write_all(&[5, 0x00, 0, 1, 0, 0, 0, 0, 0, 0])
        .await?;

    let (mut client_read, mut client_write) = tokio::io::split(client_stream);
    let (mut tor_read, mut tor_write) = tokio::io::split(tor_stream);

    let client_to_tor = tokio::io::copy(&mut client_read, &mut tor_write);
    let tor_to_client = tokio::io::copy(&mut tor_read, &mut client_write);

    tokio::try_join!(client_to_tor, tor_to_client)?;

    Ok(())
}
