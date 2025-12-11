# Error Handling Strategy

---

## 1. Error Type Definitions (error.rs)

```rust
use thiserror::Error;

/// Application-level error
#[derive(Debug, Error)]
pub enum AppError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Channel send error")]
    ChannelSend,

    #[error("Room not found: {0}")]
    RoomNotFound(String),

    #[error("Room is full")]
    RoomFull,

    #[error("Username required")]
    UsernameRequired,

    #[error("Not in room")]
    NotInRoom,

    #[error("Already in room")]
    AlreadyInRoom,
}

/// Message send error
#[derive(Debug, Error)]
pub enum SendError {
    #[error("Channel closed")]
    ChannelClosed,
}
```

---

## 2. Error Classification

### 2.1 Fatal Errors (Connection Termination)

Errors that prevent connection maintenance:

| Error | Description | Action |
|-------|-------------|--------|
| `WebSocket` | WS protocol error | Close connection |
| `Io` | Network error | Close connection |
| `ChannelSend` | Internal channel broken | Close connection |

### 2.2 Business Errors (Send Error Message)

Notify client and continue:

| Error | Description | ErrorCode |
|-------|-------------|-----------|
| `RoomNotFound` | Room code doesn't exist | `room_not_found` |
| `RoomFull` | Room capacity exceeded | `room_full` |
| `UsernameRequired` | Username needed | `username_required` |
| `NotInRoom` | Not in a room | `not_in_room` |
| `AlreadyInRoom` | Already in a room | `already_in_room` |
| `Json` | JSON parsing failed | `invalid_message` |

---

## 3. Error Handling Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          ERROR HANDLING FLOW                            │
└─────────────────────────────────────────────────────────────────────────┘

     ┌───────────────┐
     │ Error Occurs  │
     └───────┬───────┘
             │
     ┌───────▼───────┐
     │ Error Type?   │
     └───────┬───────┘
             │
   ┌─────────┼─────────────────────────────────┐
   │         │                                 │
   ▼         ▼                                 ▼
┌────────────────────┐              ┌─────────────────────────┐
│   Fatal Error      │              │    Business Error       │
│                    │              │                         │
│ • WebSocket        │              │ • RoomNotFound          │
│ • Io               │              │ • RoomFull              │
│ • ChannelSend      │              │ • UsernameRequired      │
│                    │              │ • NotInRoom             │
│                    │              │ • Json (parsing)        │
└─────────┬──────────┘              └────────────┬────────────┘
          │                                      │
          ▼                                      ▼
┌────────────────────┐              ┌─────────────────────────┐
│  Close Connection  │              │  Send Error Message     │
│                    │              │  to Client              │
│  1. Send Disconnect│              │                         │
│     command        │              │  ServerMessage::Error   │
│  2. Clean up       │              │  { code, message }      │
│     resources      │              │                         │
│  3. End Handler    │              │                         │
└────────────────────┘              └────────────┬────────────┘
                                                 │
                                                 ▼
                                    ┌─────────────────────────┐
                                    │  Continue Normal        │
                                    │  Operation              │
                                    │                         │
                                    │  (keep connection)      │
                                    └─────────────────────────┘
```

---

## 4. Error → ServerMessage Conversion

```rust
impl From<AppError> for ServerMessage {
    fn from(err: AppError) -> Self {
        let (code, message) = match &err {
            AppError::UsernameRequired => {
                (ErrorCode::UsernameRequired, "Username is required".to_string())
            }
            AppError::RoomNotFound(room_code) => {
                (ErrorCode::RoomNotFound, format!("Room '{}' not found", room_code))
            }
            AppError::RoomFull => {
                (ErrorCode::RoomFull, "Room is full".to_string())
            }
            AppError::NotInRoom => {
                (ErrorCode::NotInRoom, "You are not in a room".to_string())
            }
            AppError::AlreadyInRoom => {
                (ErrorCode::AlreadyInRoom, "You are already in a room".to_string())
            }
            AppError::Json(e) => {
                (ErrorCode::InvalidMessage, format!("Invalid message format: {}", e))
            }
            // Fatal errors are not converted (connection closes)
            _ => {
                (ErrorCode::InvalidMessage, "Internal error".to_string())
            }
        };
        ServerMessage::Error { code, message }
    }
}
```

---

## 5. Error Handling Pattern in Handler

```rust
// handler.rs

pub async fn handle_connection(
    stream: TcpStream,
    cmd_tx: mpsc::Sender<ServerCommand>,
) -> Result<(), AppError> {
    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let client_id = ClientId::new();
    let (msg_tx, mut msg_rx) = mpsc::channel::<ServerMessage>(32);

    // Register connection with server
    cmd_tx.send(ServerCommand::Connect {
        client_id,
        sender: msg_tx,
    }).await.map_err(|_| AppError::ChannelSend)?;

    // Send connection success message
    let connected_msg = serde_json::to_string(&ServerMessage::Connected {
        client_id: client_id.to_string(),
    })?;
    ws_sender.send(Message::Text(connected_msg)).await?;

    // Split Read/Write
    let read_handle = tokio::spawn(async move {
        while let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    // JSON parsing error handled as business error
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(client_msg) => {
                            let cmd = client_message_to_command(client_id, client_msg);
                            if cmd_tx.send(cmd).await.is_err() {
                                break; // Server shutdown
                            }
                        }
                        Err(e) => {
                            // Parsing error: send error message and continue
                            let err_msg = ServerMessage::Error {
                                code: ErrorCode::InvalidMessage,
                                message: format!("Invalid JSON: {}", e),
                            };
                            // Send error via msg_tx (needs separate handling)
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break, // Fatal error
                _ => {} // Ping/Pong handled by library
            }
        }
    });

    // Write loop
    let write_handle = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(_) => continue, // Serialization failure (rare)
            };
            if ws_sender.send(Message::Text(json)).await.is_err() {
                break; // Connection lost
            }
        }
    });

    // Clean up when either finishes
    tokio::select! {
        _ = read_handle => {}
        _ = write_handle => {}
    }

    // Disconnect command
    let _ = cmd_tx.send(ServerCommand::Disconnect { client_id }).await;

    Ok(())
}
```

---

## 6. Error Handling Pattern in ChatServer

```rust
// server.rs

impl ChatServer {
    async fn handle_create_room(&mut self, client_id: ClientId) {
        // Verify client exists
        let client = match self.clients.get(&client_id) {
            Some(c) => c,
            None => return, // Client not found (abnormal situation)
        };

        // Check username
        if client.username.is_none() {
            let _ = client.send(AppError::UsernameRequired.into()).await;
            return;
        }

        // Check if already in a room
        if self.client_rooms.contains_key(&client_id) {
            let _ = client.send(AppError::AlreadyInRoom.into()).await;
            return;
        }

        // Create room
        let room_code = RoomCode::generate();
        let room = Room::new(room_code.clone(), client_id);

        self.rooms.insert(room_code.clone(), room);
        self.client_rooms.insert(client_id, room_code.clone());

        let _ = client.send(ServerMessage::RoomCreated {
            room_code: room_code.to_string(),
        }).await;
    }

    async fn handle_join_room(&mut self, client_id: ClientId, room_code: String) {
        let client = match self.clients.get(&client_id) {
            Some(c) => c,
            None => return,
        };

        // Check username
        if client.username.is_none() {
            let _ = client.send(AppError::UsernameRequired.into()).await;
            return;
        }

        // Check if already in a room
        if self.client_rooms.contains_key(&client_id) {
            let _ = client.send(AppError::AlreadyInRoom.into()).await;
            return;
        }

        let room_code = RoomCode::from_string(room_code);

        // Check room exists
        let room = match self.rooms.get_mut(&room_code) {
            Some(r) => r,
            None => {
                let _ = client.send(AppError::RoomNotFound(room_code.to_string()).into()).await;
                return;
            }
        };

        // Check room capacity
        if room.is_full() {
            let _ = client.send(AppError::RoomFull.into()).await;
            return;
        }

        // Process join
        let host_id = room.host;
        room.guest = Some(client_id);
        self.client_rooms.insert(client_id, room_code.clone());

        // Get host name
        let host_name = self.clients.get(&host_id)
            .and_then(|c| c.username.clone());

        // Notify joiner
        let _ = client.send(ServerMessage::RoomJoined {
            room_code: room_code.to_string(),
            partner: host_name,
        }).await;

        // Notify host
        if let Some(host) = self.clients.get(&host_id) {
            let guest_name = client.username.clone().unwrap_or_default();
            let _ = host.send(ServerMessage::PartnerJoined {
                username: guest_name,
            }).await;
        }
    }
}
```

---

## 7. Result Chaining Example

```rust
/// Helper function to send message to partner
async fn send_to_partner(
    &self,
    client_id: ClientId,
    msg: ServerMessage,
) -> Result<(), AppError> {
    let room_code = self.client_rooms
        .get(&client_id)
        .ok_or(AppError::NotInRoom)?;

    let room = self.rooms
        .get(room_code)
        .ok_or(AppError::NotInRoom)?;

    let partner_id = room
        .get_partner(client_id)
        .ok_or(AppError::NotInRoom)?;

    let partner = self.clients
        .get(&partner_id)
        .ok_or(AppError::ChannelSend)?;

    partner.send(msg).await
        .map_err(|_| AppError::ChannelSend)
}
```

---

## 8. Error Logging

```rust
use tracing::{info, warn, error};

// Fatal error
error!("WebSocket error for client {}: {}", client_id, err);

// Business error
warn!("Client {} tried to join non-existent room: {}", client_id, room_code);

// Normal operation
info!("Client {} created room {}", client_id, room_code);
```
