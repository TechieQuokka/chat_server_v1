# 1:1 WebSocket Chat Server

A learning-oriented 1:1 WebSocket chat server built with Rust, tokio-tungstenite, and the Actor pattern.

## Features

- **WebSocket Communication**: Real-time bidirectional messaging
- **Room System**: Create and join rooms using 6-character codes
- **Typing Indicators**: See when your chat partner is typing
- **Actor Pattern**: Lock-free state management using mpsc channels
- **In-Memory Storage**: No database required (learning-focused)

## Architecture

```
┌─────────────────┐          ┌─────────────────┐
│   Client A      │          │   Client B      │
│   (WebSocket)   │          │   (WebSocket)   │
└────────┬────────┘          └────────┬────────┘
         │                            │
         │    ServerCommand (mpsc)    │
         └──────────┬─────────────────┘
                    │
           ┌────────▼────────┐
           │   ChatServer    │
           │    (Actor)      │
           │                 │
           │  clients: HashMap
           │  rooms: HashMap │
           │  client_rooms   │
           └─────────────────┘
```

### Why Actor Pattern?

| Pattern | Pros | Cons |
|---------|------|------|
| `Arc<Mutex<T>>` | Simple | Lock contention, deadlock risk |
| `Arc<RwLock<T>>` | Good read performance | Still lock-based |
| **Actor (mpsc)** | No locks, message-based, scalable | Slightly complex |

## Tech Stack

| Category | Technology |
|----------|------------|
| Runtime | tokio |
| WebSocket | tokio-tungstenite |
| Serialization | serde, serde_json |
| ID Generation | uuid, rand |
| Error Handling | thiserror |
| Logging | tracing |

## Getting Started

### Prerequisites

- Rust 1.70+ (2021 edition)

### Build & Run

```bash
# Development build
cargo build

# Run server (default: 127.0.0.1:8080)
cargo run

# Custom address
cargo run 0.0.0.0:9000

# With debug logging
RUST_LOG=debug cargo run
```

### Run Tests

```bash
cargo test
```

## Message Protocol

### Client → Server

```json
// Set username (required first)
{ "type": "set_username", "username": "Alice" }

// Create room
{ "type": "create_room" }

// Join room
{ "type": "join_room", "room_code": "ABC123" }

// Send message
{ "type": "chat", "content": "Hello!" }

// Typing indicators
{ "type": "typing" }
{ "type": "stop_typing" }

// Leave room
{ "type": "leave_room" }
```

### Server → Client

```json
// Connection successful
{ "type": "connected", "client_id": "uuid-here" }

// Username set
{ "type": "username_set", "username": "Alice" }

// Room created
{ "type": "room_created", "room_code": "ABC123" }

// Room joined
{ "type": "room_joined", "room_code": "ABC123", "partner": "Bob" }

// Partner joined
{ "type": "partner_joined", "username": "Bob" }

// Chat message
{ "type": "chat", "from": "Alice", "content": "Hello!" }

// Typing indicators
{ "type": "partner_typing" }
{ "type": "partner_stop_typing" }

// Partner left
{ "type": "partner_left" }

// Error
{ "type": "error", "code": "room_not_found", "message": "Room 'XYZ' not found" }
```

## Project Structure

```
src/
├── main.rs      # Entry point, TCP listener
├── lib.rs       # Module declarations, re-exports
├── types.rs     # ClientId, RoomCode (newtype pattern)
├── message.rs   # ClientMessage, ServerMessage, ErrorCode
├── client.rs    # Client struct
├── room.rs      # Room struct
├── server.rs    # ChatServer actor, ServerCommand
├── handler.rs   # WebSocket connection handler
└── error.rs     # AppError, SendError
```

## Documentation

Detailed design documents are available in:
- `documents/` - English documentation
- `documents_kr/` - Korean documentation

## License

MIT
