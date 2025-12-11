# Project Structure

---

## 1. Directory Structure

```
chat_server_v1/
├── Cargo.toml              # Project metadata and dependencies
├── Cargo.lock              # Dependency version lock
├── documents/              # Design documents (Korean)
│   ├── architecture.md
│   ├── data-structures.md
│   ├── message-protocol.md
│   ├── sequence-diagrams.md
│   ├── error-handling.md
│   └── project-structure.md
├── documents_en/           # Design documents (English)
│   ├── architecture.md
│   ├── data-structures.md
│   ├── message-protocol.md
│   ├── sequence-diagrams.md
│   ├── error-handling.md
│   └── project-structure.md (this file)
└── src/
    ├── main.rs             # Entry point: TCP Listener, server startup
    ├── lib.rs              # Module declarations and re-exports
    ├── types.rs            # ClientId, RoomCode (newtype)
    ├── message.rs          # ClientMessage, ServerMessage, ErrorCode
    ├── client.rs           # Client struct
    ├── room.rs             # Room struct
    ├── server.rs           # ChatServer Actor, ServerCommand
    ├── handler.rs          # WebSocket connection handler
    └── error.rs            # AppError, SendError
```

---

## 2. Cargo.toml

```toml
[package]
name = "chat_server_v1"
version = "0.1.0"
edition = "2021"
authors = ["Your Name"]
description = "A simple 1:1 WebSocket chat server using tokio-tungstenite"

[dependencies]
# Async runtime
tokio = { version = "1.41", features = ["full"] }

# WebSocket
tokio-tungstenite = "0.24"

# Futures utilities (StreamExt, SinkExt)
futures-util = { version = "0.3", default-features = false, features = ["sink", "std"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# UUID generation
uuid = { version = "1.11", features = ["v4"] }

# Random generation (for room code)
rand = "0.8"

# Error handling
thiserror = "2.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
lto = true
codegen-units = 1
```

---

## 3. Module Responsibilities

### 3.1 main.rs
- Application entry point
- tracing initialization
- TcpListener binding
- ChatServer Actor startup
- Connection accept loop

```rust
// Main flow
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("chat_server_v1=debug,info")
        .init();

    // 2. Start TCP listener
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    // 3. Create ChatServer Actor channel and start
    let (cmd_tx, cmd_rx) = mpsc::channel(256);
    let server = ChatServer::new(cmd_rx);
    tokio::spawn(server.run());

    // 4. Connection accept loop
    loop {
        let (stream, _) = listener.accept().await?;
        let cmd_tx = cmd_tx.clone();
        tokio::spawn(handle_connection(stream, cmd_tx));
    }
}
```

### 3.2 lib.rs
- Module declarations
- Main type re-exports

```rust
pub mod types;
pub mod message;
pub mod client;
pub mod room;
pub mod server;
pub mod handler;
pub mod error;

pub use types::{ClientId, RoomCode};
pub use message::{ClientMessage, ServerMessage, ErrorCode};
pub use client::Client;
pub use room::Room;
pub use server::{ChatServer, ServerCommand};
pub use error::{AppError, SendError};
```

### 3.3 types.rs
- Basic type definitions
- `ClientId`: UUID-based client identifier
- `RoomCode`: 6-character alphanumeric room code

### 3.4 message.rs
- Protocol message definitions
- `ClientMessage`: Client → Server
- `ServerMessage`: Server → Client
- `ErrorCode`: Error code enum

### 3.5 client.rs
- `Client` struct
- Client state (id, username, sender, is_typing)
- Message send helper methods

### 3.6 room.rs
- `Room` struct
- Room state (code, host, guest, created_at)
- Room management methods (is_full, get_partner, remove_client)

### 3.7 server.rs
- `ChatServer` Actor
- `ServerCommand` enum
- State management (clients, rooms, client_rooms)
- Command handlers

### 3.8 handler.rs
- WebSocket connection handler
- WS handshake
- Read/Write task separation
- ClientMessage → ServerCommand conversion

### 3.9 error.rs
- `AppError`: Application errors
- `SendError`: Channel send errors
- Error → ServerMessage conversion

---

## 4. Data Flow Summary

```
                    ┌─────────────────────────────────┐
                    │           main.rs               │
                    │   TcpListener.accept()          │
                    └───────────────┬─────────────────┘
                                    │
                                    │ TcpStream
                                    ▼
                    ┌─────────────────────────────────┐
                    │          handler.rs             │
                    │                                 │
                    │  ┌─────────────────────────┐    │
                    │  │ WebSocket Handshake     │    │
                    │  └─────────────────────────┘    │
                    │                                 │
                    │  ┌─────────────────────────┐    │
                    │  │ Read Task               │    │
                    │  │ WS → ClientMessage      │    │
                    │  │ → ServerCommand         │───────────┐
                    │  └─────────────────────────┘    │       │
                    │                                 │       │
                    │  ┌─────────────────────────┐    │       │
                    │  │ Write Task              │◄───────────┼──┐
                    │  │ ServerMessage → WS      │    │       │  │
                    │  └─────────────────────────┘    │       │  │
                    └─────────────────────────────────┘       │  │
                                                              │  │
                              mpsc::Sender<ServerCommand>     │  │
                                                              │  │
                    ┌─────────────────────────────────────────▼──┤
                    │              server.rs                     │
                    │            ChatServer Actor                │
                    │                                            │
                    │  ┌────────────────────────────────────┐    │
                    │  │ clients: HashMap<ClientId, Client> │    │
                    │  │ rooms: HashMap<RoomCode, Room>     │    │
                    │  │ client_rooms: HashMap<ClientId,    │    │
                    │  │               RoomCode>            │    │
                    │  └────────────────────────────────────┘    │
                    │                                            │
                    │  handle_command() → ServerMessage ─────────┘
                    │                                            │
                    └────────────────────────────────────────────┘
                                        │
                                        │ uses
                    ┌───────────────────┼───────────────────┐
                    │                   │                   │
                    ▼                   ▼                   ▼
            ┌───────────────┐   ┌───────────────┐   ┌───────────────┐
            │   client.rs   │   │   room.rs     │   │  message.rs   │
            │               │   │               │   │               │
            │   Client      │   │   Room        │   │ ClientMessage │
            │               │   │               │   │ ServerMessage │
            └───────────────┘   └───────────────┘   └───────────────┘
                    │                   │                   │
                    └───────────────────┼───────────────────┘
                                        │
                                        ▼
                                ┌───────────────┐
                                │   types.rs    │
                                │   error.rs    │
                                └───────────────┘
```

---

## 5. Build and Run

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run
cargo run

# Set log level
RUST_LOG=debug cargo run
RUST_LOG=chat_server_v1=trace cargo run
```

---

## 6. Testing (Future Extension)

```
src/
└── tests/
    ├── integration_test.rs   # Integration tests
    └── unit/
        ├── room_test.rs      # Room unit tests
        └── message_test.rs   # Message serialization tests
```

```bash
# Run tests
cargo test

# Run specific test
cargo test room_test
```
