# Data Structures

---

## 1. Type Definitions (types.rs)

### 1.1 ClientId

Unique client identifier. Uses the newtype pattern for type safety.

```rust
use uuid::Uuid;

/// Unique client identifier (newtype pattern)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub Uuid);

impl ClientId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ClientId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

### 1.2 RoomCode

Room code (6-character uppercase alphanumeric).

```rust
/// Room code (6-character uppercase alphanumeric)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoomCode(pub String);

impl RoomCode {
    pub fn generate() -> Self {
        use rand::Rng;
        let code: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(6)
            .map(char::from)
            .collect::<String>()
            .to_uppercase();
        Self(code)
    }

    pub fn from_string(code: String) -> Self {
        Self(code.to_uppercase())
    }
}

impl std::fmt::Display for RoomCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

---

## 2. Client Struct (client.rs)

```rust
use crate::types::ClientId;
use crate::message::ServerMessage;
use crate::error::SendError;
use tokio::sync::mpsc;

/// Connected client information
#[derive(Debug)]
pub struct Client {
    /// Unique identifier
    pub id: ClientId,
    /// Username (None before setup)
    pub username: Option<String>,
    /// Server → Client message channel
    pub sender: mpsc::Sender<ServerMessage>,
    /// Currently typing flag
    pub is_typing: bool,
}

impl Client {
    pub fn new(id: ClientId, sender: mpsc::Sender<ServerMessage>) -> Self {
        Self {
            id,
            username: None,
            sender,
            is_typing: false,
        }
    }

    /// Send message to client
    pub async fn send(&self, msg: ServerMessage) -> Result<(), SendError> {
        self.sender
            .send(msg)
            .await
            .map_err(|_| SendError::ChannelClosed)
    }

    /// Get display name (returns "Unknown" if not set)
    pub fn display_name(&self) -> &str {
        self.username.as_deref().unwrap_or("Unknown")
    }
}
```

---

## 3. Room Struct (room.rs)

```rust
use crate::types::{ClientId, RoomCode};
use std::time::Instant;

/// 1:1 Chat Room
#[derive(Debug)]
pub struct Room {
    /// Room code
    pub code: RoomCode,
    /// Room creator (host)
    pub host: ClientId,
    /// Joined partner (guest)
    pub guest: Option<ClientId>,
    /// Room creation time
    pub created_at: Instant,
}

impl Room {
    pub fn new(code: RoomCode, host: ClientId) -> Self {
        Self {
            code,
            host,
            guest: None,
            created_at: Instant::now(),
        }
    }

    /// Check if room is full (2 people)
    pub fn is_full(&self) -> bool {
        self.guest.is_some()
    }

    /// Check if room is empty (only host)
    pub fn is_empty(&self) -> bool {
        self.guest.is_none()
    }

    /// Get partner's ClientId
    pub fn get_partner(&self, client_id: ClientId) -> Option<ClientId> {
        if self.host == client_id {
            self.guest
        } else if self.guest == Some(client_id) {
            Some(self.host)
        } else {
            None
        }
    }

    /// Check if client is in the room
    pub fn contains(&self, client_id: ClientId) -> bool {
        self.host == client_id || self.guest == Some(client_id)
    }

    /// Remove client (handle leaving)
    /// Returns: whether the room should be deleted
    pub fn remove_client(&mut self, client_id: ClientId) -> bool {
        if self.host == client_id {
            // If host leaves, promote guest to host
            if let Some(guest) = self.guest.take() {
                self.host = guest;
                false // Keep room
            } else {
                true // Delete room
            }
        } else if self.guest == Some(client_id) {
            self.guest = None;
            false // Keep room
        } else {
            false
        }
    }
}
```

---

## 4. State Relationship Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ChatServer State                            │
└─────────────────────────────────────────────────────────────────────┘

  clients: HashMap<ClientId, Client>
  ┌─────────────────────────────────────────────────────────────────┐
  │                                                                 │
  │  ClientId(uuid1) ──► Client {                                   │
  │                        id: uuid1,                               │
  │                        username: Some("Alice"),                 │
  │                        sender: mpsc::Sender,                    │
  │                        is_typing: false                         │
  │                      }                                          │
  │                                                                 │
  │  ClientId(uuid2) ──► Client {                                   │
  │                        id: uuid2,                               │
  │                        username: Some("Bob"),                   │
  │                        sender: mpsc::Sender,                    │
  │                        is_typing: true                          │
  │                      }                                          │
  │                                                                 │
  └─────────────────────────────────────────────────────────────────┘

  rooms: HashMap<RoomCode, Room>
  ┌─────────────────────────────────────────────────────────────────┐
  │                                                                 │
  │  RoomCode("ABC123") ──► Room {                                  │
  │                           code: "ABC123",                       │
  │                           host: uuid1,      ◄─── Alice          │
  │                           guest: Some(uuid2), ◄─── Bob          │
  │                           created_at: Instant                   │
  │                         }                                       │
  │                                                                 │
  │  RoomCode("XYZ789") ──► Room {                                  │
  │                           code: "XYZ789",                       │
  │                           host: uuid3,                          │
  │                           guest: None,      ◄─── Waiting        │
  │                           created_at: Instant                   │
  │                         }                                       │
  │                                                                 │
  └─────────────────────────────────────────────────────────────────┘

  client_rooms: HashMap<ClientId, RoomCode>  (for fast lookup)
  ┌─────────────────────────────────────────────────────────────────┐
  │                                                                 │
  │  ClientId(uuid1) ──► RoomCode("ABC123")                         │
  │  ClientId(uuid2) ──► RoomCode("ABC123")                         │
  │  ClientId(uuid3) ──► RoomCode("XYZ789")                         │
  │                                                                 │
  └─────────────────────────────────────────────────────────────────┘
```

---

## 5. Benefits of Newtype Pattern

### Type Safety
```rust
// Bad: Can accidentally use wrong type
fn join_room(client_id: Uuid, room_code: String) { ... }

// Good: Compile-time type checking
fn join_room(client_id: ClientId, room_code: RoomCode) { ... }
```

### Method Encapsulation
```rust
// RoomCode-specific methods
impl RoomCode {
    pub fn generate() -> Self { ... }
    pub fn is_valid(&self) -> bool { ... }
}
```

### Automatic Hash/Eq Implementation
```rust
#[derive(Hash, Eq, PartialEq)]
pub struct ClientId(pub Uuid);

// Can be used directly as HashMap key
let clients: HashMap<ClientId, Client> = HashMap::new();
```
