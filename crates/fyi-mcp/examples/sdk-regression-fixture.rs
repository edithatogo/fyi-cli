//! Standalone SDK fixture; the production server's entrypoint is not changed.
//!
//! Include the real server so the SDK uses its stdio framing, MCP dispatch,
//! policy, telemetry and source handling. Only the source transport is faked.
#![allow(dead_code)]

#[path = "../src/main.rs"]
mod server;
// Modules in the included server use the binary crate's normal module paths.
pub use server::{policy, write_journal};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    server::run_sdk_regression_fixture().await
}
