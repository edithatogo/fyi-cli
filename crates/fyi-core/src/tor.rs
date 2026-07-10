use crate::agent_runtime::{
    AgentNetworkMiddleware, AgentRuntimeError, ClientIdentity, GuardrailConfig,
    OutboundRequestMeta, PacingPolicy,
};
use arti_client::{config::TorClientConfigBuilder, TorClient, TorClientConfig};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::{Client, RequestBuilder, StatusCode};
use std::net::IpAddr;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::{oneshot, Mutex, Semaphore};
use tor_rtcompat::PreferredRuntime;

#[derive(thiserror::Error, Debug)]
pub enum TorError {
    #[error("Arti client error: {0}")]
    Arti(#[from] arti_client::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tor runtime error: {0}")]
    Runtime(String),

    #[error("Tor agent request blocked: {0}")]
    Agent(String),

    #[error("SOCKS handshake protocol error: {0}")]
    Protocol(String),
}

#[derive(Debug, Clone)]
pub struct TorBootstrapStatus {
    pub ready: bool,
    pub progress: f32,
}

/// Response returned by the Tor-routed resource-aware executor.
#[derive(Debug, Clone)]
pub struct TorGuardedResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

/// Tor transport paired with the same identity, pacing, guardrail, memory,
/// cache, and trace middleware used by the ordinary Rust sync client.
pub struct TorAgentClient {
    client: Client,
    middleware: Arc<Mutex<AgentNetworkMiddleware>>,
    concurrency: Arc<Semaphore>,
    instance_id: String,
}

impl TorAgentClient {
    pub async fn user_agent(&self) -> String {
        self.middleware.lock().await.user_agent()
    }

    pub async fn execute(&self, request: RequestBuilder) -> Result<TorGuardedResponse, TorError> {
        let request = request
            .build()
            .map_err(|error| TorError::Agent(format!("failed to build request: {error}")))?;
        let meta = OutboundRequestMeta {
            instance_id: self.instance_id.clone(),
            route_class: "tor_alaveteli_api".to_string(),
            method: request.method().to_string(),
            url: request.url().to_string(),
        };
        let decision = self
            .middleware
            .lock()
            .await
            .before_request(&meta)
            .map_err(|error| TorError::Agent(error.to_string()))?;
        if !decision.wait.is_zero() {
            tokio::time::sleep(decision.wait).await;
        }
        let _permit = self
            .concurrency
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| TorError::Agent("concurrency guard unavailable".to_string()))?;
        let started = Instant::now();
        let response = self
            .client
            .execute(request)
            .await
            .map_err(|error| TorError::Agent(format!("Tor request failed: {error}")))?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .bytes()
            .await
            .map_err(|error| TorError::Agent(format!("Tor response read failed: {error}")))?
            .to_vec();
        let header_values = headers
            .iter()
            .filter_map(|(name, value)| value.to_str().ok().map(|value| (name.as_str(), value)))
            .collect::<Vec<_>>();
        self.middleware
            .lock()
            .await
            .after_response(
                &meta,
                status.as_u16(),
                &header_values,
                started.elapsed(),
                Some(&body),
            )
            .map_err(|error| match error {
                AgentRuntimeError::RateLimited(seconds) => {
                    TorError::Agent(format!("rate limited; retry after {seconds} seconds"))
                }
                other => TorError::Agent(other.to_string()),
            })?;
        Ok(TorGuardedResponse {
            status,
            headers,
            body,
        })
    }
}

pub struct TorManager {
    client: TorClient<PreferredRuntime>,
    proxy_addr: Option<SocketAddr>,
}

impl TorManager {
    pub fn new() -> Result<Self, TorError> {
        let config = TorClientConfig::default();
        Self::with_config(config)
    }

    pub fn new_with_storage_dirs(
        state_dir: impl AsRef<Path>,
        cache_dir: impl AsRef<Path>,
    ) -> Result<Self, TorError> {
        let config = TorClientConfigBuilder::from_directories(state_dir, cache_dir)
            .build()
            .map_err(|e| TorError::Runtime(e.to_string()))?;
        Self::with_config(config)
    }

    fn with_config(config: TorClientConfig) -> Result<Self, TorError> {
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
        let identity = crate::agent_runtime::ClientIdentity::default_identity(None)
            .map_err(|e| TorError::Runtime(format!("Invalid client identity: {e}")))?;
        self.create_reqwest_client_with_identity(&identity)
    }

    /// Build the Tor-routed client with the mandatory traceable User-Agent.
    pub fn create_reqwest_client_with_identity(
        &self,
        identity: &crate::agent_runtime::ClientIdentity,
    ) -> Result<reqwest::Client, TorError> {
        identity
            .validate()
            .map_err(|e| TorError::Runtime(format!("Invalid client identity: {e}")))?;
        let proxy_addr = self.proxy_addr.ok_or_else(|| {
            TorError::Runtime("Proxy has not been started yet. Call start_proxy first.".to_string())
        })?;

        let proxy_url = format!("socks5h://{}", proxy_addr);
        let proxy = reqwest::Proxy::all(&proxy_url)
            .map_err(|e| TorError::Runtime(format!("Failed to parse proxy URL: {}", e)))?;

        let default_headers = identity_headers(identity)?;
        let client = reqwest::Client::builder()
            .proxy(proxy)
            .default_headers(default_headers)
            .build()
            .map_err(|e| TorError::Runtime(format!("Failed to build reqwest Client: {}", e)))?;

        Ok(client)
    }

    /// Build the Tor client together with the shared resource-aware executor.
    pub fn create_guarded_client(
        &self,
        identity: &ClientIdentity,
    ) -> Result<TorAgentClient, TorError> {
        let client = self.create_reqwest_client_with_identity(identity)?;
        let guardrails = GuardrailConfig::default();
        Ok(TorAgentClient {
            client,
            middleware: Arc::new(Mutex::new(AgentNetworkMiddleware::new(
                identity.clone(),
                guardrails.clone(),
                PacingPolicy::default(),
                Box::new(crate::agent_runtime::NullTraceSink),
                "tor-agent".to_string(),
            ))),
            concurrency: Arc::new(Semaphore::new(guardrails.max_concurrency as usize)),
            instance_id: "tor-instance".to_string(),
        })
    }
}

fn identity_headers(
    identity: &crate::agent_runtime::ClientIdentity,
) -> Result<HeaderMap, TorError> {
    let user_agent = identity.user_agent();
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&user_agent)
            .map_err(|e| TorError::Runtime(format!("Invalid User-Agent: {e}")))?,
    );
    Ok(headers)
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

#[cfg(test)]
mod tests {
    use super::identity_headers;

    #[test]
    fn identity_headers_include_traceable_user_agent() {
        let identity = crate::agent_runtime::ClientIdentity::custom(
            "fyi-test",
            "9.9.9",
            "https://example.test/fyi",
            Some("ops@example.test".to_string()),
        )
        .expect("test identity should validate");

        let headers = identity_headers(&identity).expect("identity headers should build");
        assert_eq!(
            headers.get(reqwest::header::USER_AGENT),
            Some(&reqwest::header::HeaderValue::from_str(&identity.user_agent()).unwrap())
        );
    }
}
