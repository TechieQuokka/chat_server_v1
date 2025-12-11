//! 1:1 WebSocket Chat Server - Entry Point
//!
//! Starts the TCP listener and ChatServer actor, accepting connections.

use std::env;

use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use chat_server_v1::{handle_connection, ChatServer};

/// Default server address
const DEFAULT_ADDR: &str = "127.0.0.1:8080";

/// Channel buffer size for server commands
const CHANNEL_BUFFER_SIZE: usize = 256;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with environment filter
    // Use RUST_LOG env var to control log level
    // e.g., RUST_LOG=debug or RUST_LOG=chat_server_v1=trace
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("chat_server_v1=info")),
        )
        .init();

    // Get bind address from command line or use default
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_ADDR.to_string());

    // Start TCP listener
    let listener = TcpListener::bind(&addr).await?;
    info!("WebSocket Chat Server listening on {}", addr);

    // Create ChatServer actor channel and start
    let (cmd_tx, cmd_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
    let server = ChatServer::new(cmd_rx);
    tokio::spawn(server.run());

    info!("ChatServer actor started");

    // Connection accept loop
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("New connection from {}", addr);
                let cmd_tx = cmd_tx.clone();

                // Spawn handler task for each connection
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, cmd_tx).await {
                        error!("Connection handler error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}
