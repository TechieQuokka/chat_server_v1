# Message Protocol

---

## 1. Overview

JSON-based bidirectional message protocol. Uses Serde's tagged enum for type-safe serialization/deserialization.

---

## 2. Client → Server Messages (ClientMessage)

```rust
use serde::{Deserialize, Serialize};

/// Client → Server message
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Set username
    SetUsername { username: String },
    /// Create new room
    CreateRoom,
    /// Join existing room
    JoinRoom { room_code: String },
    /// Send chat message
    Chat { content: String },
    /// Start typing
    Typing,
    /// Stop typing
    StopTyping,
    /// Leave room
    LeaveRoom,
}
```

### JSON Examples

```json
// Set username
{ "type": "set_username", "username": "Alice" }

// Create room
{ "type": "create_room" }

// Join room
{ "type": "join_room", "room_code": "ABC123" }

// Chat message
{ "type": "chat", "content": "Hello!" }

// Start typing
{ "type": "typing" }

// Stop typing
{ "type": "stop_typing" }

// Leave room
{ "type": "leave_room" }
```

---

## 3. Server → Client Messages (ServerMessage)

```rust
/// Server → Client message
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Connection successful, client ID issued
    Connected { client_id: String },
    /// Username set successfully
    UsernameSet { username: String },
    /// Room created successfully
    RoomCreated { room_code: String },
    /// Room joined successfully
    RoomJoined {
        room_code: String,
        partner: Option<String>,
    },
    /// Partner joined
    PartnerJoined { username: String },
    /// Chat message received
    Chat { from: String, content: String },
    /// Partner is typing
    PartnerTyping,
    /// Partner stopped typing
    PartnerStopTyping,
    /// Partner left
    PartnerLeft,
    /// Error
    Error { code: ErrorCode, message: String },
}
```

### JSON Examples

```json
// Connection successful
{ "type": "connected", "client_id": "550e8400-e29b-41d4-a716-446655440000" }

// Username set successfully
{ "type": "username_set", "username": "Alice" }

// Room created successfully
{ "type": "room_created", "room_code": "ABC123" }

// Room joined (with partner)
{ "type": "room_joined", "room_code": "ABC123", "partner": "Alice" }

// Room joined (no partner, waiting)
{ "type": "room_joined", "room_code": "ABC123", "partner": null }

// Partner joined
{ "type": "partner_joined", "username": "Bob" }

// Chat message received
{ "type": "chat", "from": "Alice", "content": "Hello!" }

// Partner is typing
{ "type": "partner_typing" }

// Partner stopped typing
{ "type": "partner_stop_typing" }

// Partner left
{ "type": "partner_left" }

// Error
{ "type": "error", "code": "room_not_found", "message": "Room 'XYZ999' not found" }
```

---

## 4. Error Codes (ErrorCode)

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Attempted action without setting username
    UsernameRequired,
    /// Non-existent room code
    RoomNotFound,
    /// Room already has 2 people
    RoomFull,
    /// Attempted chat without joining a room
    NotInRoom,
    /// Invalid message format
    InvalidMessage,
}
```

### Error Scenarios

| Error Code | Trigger Scenario |
|------------|------------------|
| `username_required` | Attempted room creation/join/chat before setting username |
| `room_not_found` | Attempted to join non-existent room code |
| `room_full` | Attempted to join room with 2 people already |
| `not_in_room` | Attempted chat/typing without being in a room |
| `invalid_message` | JSON parsing failure or unknown message type |

---

## 5. Communication Sequences

### 5.1 Basic Connection Flow

```
Client                                    Server
  │                                          │
  │──────── TCP Connect ────────────────────>│
  │                                          │
  │<─────── WS Handshake ───────────────────>│
  │                                          │
  │<──────── Connected ──────────────────────│
  │          { client_id }                   │
  │                                          │
  │──────── SetUsername ────────────────────>│
  │         { username: "Alice" }            │
  │                                          │
  │<──────── UsernameSet ────────────────────│
  │          { username: "Alice" }           │
  │                                          │
```

### 5.2 Room Creation and Joining

```
Client A                  Server                  Client B
  │                          │                        │
  │─── CreateRoom ──────────>│                        │
  │                          │                        │
  │<── RoomCreated ──────────│                        │
  │    { room_code: "ABC" }  │                        │
  │                          │                        │
  │    (A shares "ABC" with B)                        │
  │                          │                        │
  │                          │<──── JoinRoom ─────────│
  │                          │      { room_code: "ABC"}
  │                          │                        │
  │                          │───── RoomJoined ──────>│
  │                          │      { partner: "Alice"}
  │                          │                        │
  │<── PartnerJoined ────────│                        │
  │    { username: "Bob" }   │                        │
  │                          │                        │
```

### 5.3 Chat and Typing

```
Client A                  Server                  Client B
  │                          │                        │
  │─── Typing ──────────────>│                        │
  │                          │───── PartnerTyping ───>│
  │                          │                        │
  │─── Chat ────────────────>│                        │
  │    { content: "Hi!" }    │                        │
  │                          │── PartnerStopTyping ──>│
  │                          │                        │
  │                          │───── Chat ────────────>│
  │                          │      { from: "Alice",  │
  │                          │        content: "Hi!" }│
  │                          │                        │
```

### 5.4 Disconnection

```
Client A                  Server                  Client B
  │                          │                        │
  │─── [Connection Lost] ───>│                        │
  X                          │                        │
                             │───── PartnerLeft ─────>│
                             │                        │
                             │  (Only B remains in room)
                             │                        │
```

---

## 6. Serde Configuration Explained

### Tagged Enum
```rust
#[serde(tag = "type")]
```
- Adds a `"type"` field to JSON to identify enum variant
- Example: `{ "type": "chat", "content": "Hello" }`

### Rename All
```rust
#[serde(rename_all = "snake_case")]
```
- Converts Rust's PascalCase to JSON's snake_case
- Example: `PartnerTyping` → `"partner_typing"`
