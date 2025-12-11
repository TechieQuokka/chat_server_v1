//! 1:1 WebSocket Chat Server Library
//!
//! A learning-oriented WebSocket chat server built with tokio-tungstenite
//! using the Actor pattern for state management.
//!
//! # Features
//! - WebSocket connection handling
//! - Username setup
//! - Room creation with 6-character codes
//! - Room joining
//! - Real-time chat messaging
//! - Typing indicators
//! - Disconnection handling
//!
//! # Architecture
//! Uses the Actor pattern with `mpsc` channels:
//! - `ChatServer` is the central actor managing all state
//! - Each connection has a `handler` task communicating with the server
//! - No locks needed - all state access goes through message passing
//!
//! # Example
//! ```ignore
//! use tokio::net::TcpListener;
//! use tokio::sync::mpsc;
//! use chat_server_v1::{ChatServer, handle_connection};
//!
//! #[tokio::main]
//! async fn main() {
//!     let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
//!     let (cmd_tx, cmd_rx) = mpsc::channel(256);
//!
//!     tokio::spawn(ChatServer::new(cmd_rx).run());
//!
//!     while let Ok((stream, _)) = listener.accept().await {
//!         let cmd_tx = cmd_tx.clone();
//!         tokio::spawn(handle_connection(stream, cmd_tx));
//!     }
//! }
//! ```

pub mod client;
pub mod error;
pub mod handler;
pub mod message;
pub mod room;
pub mod server;
pub mod types;

// Re-export main types for convenience
pub use client::Client;
pub use error::{AppError, SendError};
pub use handler::handle_connection;
pub use message::{ClientMessage, ErrorCode, ServerMessage};
pub use room::Room;
pub use server::{ChatServer, ServerCommand};
pub use types::{ClientId, RoomCode};
