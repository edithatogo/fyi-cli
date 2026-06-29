use fyi_core::tor::TorManager;
use std::io::ErrorKind;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::test]
async fn test_tor_initialization_and_unbootstrapped_status() {
    // 1. Initialize TorManager
    let manager = TorManager::new().expect("Failed to initialize TorManager");

    // 2. Check initial bootstrap status (should not be ready since it's unbootstrapped)
    let status = manager.get_bootstrap_status();
    assert!(
        !status.ready,
        "Client should not be ready for traffic immediately"
    );
    assert!(status.progress >= 0.0 && status.progress <= 1.0);
}

#[tokio::test]
async fn test_start_proxy_and_reqwest_client_setup() {
    let mut manager = TorManager::new().expect("Failed to initialize TorManager");

    // Start the local SOCKS proxy
    let (addr, _shutdown_tx) = manager.start_proxy().await.expect("Failed to start proxy");
    assert_ne!(addr.port(), 0, "Proxy should bind to a non-zero port");

    // Create reqwest client with proxy
    let reqwest_client = manager.create_reqwest_client();
    assert!(
        reqwest_client.is_ok(),
        "Should successfully construct reqwest client configured with proxy"
    );
}

#[tokio::test]
async fn test_socks_proxy_handshake_invalid_version() {
    let mut manager = TorManager::new().expect("Failed to initialize TorManager");
    let (addr, _shutdown_tx) = manager.start_proxy().await.expect("Failed to start proxy");

    // Connect directly to the proxy to test handshake failure
    let mut socket = TcpStream::connect(addr)
        .await
        .expect("Failed to connect to proxy port");

    // Write invalid version (e.g. SOCKS4 = 4)
    socket.write_all(&[4, 1, 0]).await.unwrap();

    // The server should drop/close connection or return error
    let mut response = [0u8; 10];
    match socket.read(&mut response).await {
        Ok(read_bytes) => assert_eq!(
            read_bytes, 0,
            "Proxy should disconnect on unsupported version"
        ),
        Err(error) if error.kind() == ErrorKind::ConnectionReset => {}
        Err(error) => panic!("Unexpected proxy read error: {error}"),
    }
}

#[tokio::test]
async fn test_socks_proxy_no_auth_required_negotiation() {
    let mut manager = TorManager::new().expect("Failed to initialize TorManager");
    let (addr, _shutdown_tx) = manager.start_proxy().await.expect("Failed to start proxy");

    let mut socket = TcpStream::connect(addr)
        .await
        .expect("Failed to connect to proxy port");

    // SOCKS5 (5), 1 method supported (1), NO AUTH (0)
    socket.write_all(&[5, 1, 0]).await.unwrap();

    let mut response = [0u8; 2];
    socket.read_exact(&mut response).await.unwrap();

    // Response should be version 5 (5), method 0 (NO AUTH)
    assert_eq!(response[0], 5);
    assert_eq!(response[1], 0);
}
