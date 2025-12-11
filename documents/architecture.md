# 1:1 WebSocket Chat Server - Architecture Design

> Built with tokio-tungstenite, Actor pattern, room code system, and typing indicator

---

## 1. Overview

### 1.1 Project Goals
- Implement a learning-oriented 1:1 WebSocket chat server
- In-memory state management without database
- Learn Rust's async/await and Actor pattern

### 1.2 Technology Stack
| Category | Technology |
|----------|------------|
| Runtime | tokio |
| WebSocket | tokio-tungstenite |
| Serialization | serde, serde_json |
| ID Generation | uuid, rand |
| Error Handling | thiserror |
| Logging | tracing |

### 1.3 Core Features
- WebSocket connection acceptance and handshake
- Username setup
- Room creation (6-character code)
- Room joining (via code)
- Real-time chat
- Typing indicator
- Disconnection handling

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                          COMPLETE ARCHITECTURE OVERVIEW                          │
└─────────────────────────────────────────────────────────────────────────────────┘

                               ┌──────────────────┐
                               │     main.rs      │
                               │  TcpListener     │
                               │  Server Boot     │
                               └────────┬─────────┘
                                        │
                     ┌──────────────────┴──────────────────┐
                     │                                     │
                     ▼                                     ▼
            ┌─────────────────┐                   ┌─────────────────┐
            │   ChatServer    │◄──────────────────│   handler.rs    │
            │    (Actor)      │   ServerCommand   │  (per client)   │
            │                 │      mpsc         │                 │
            │  ┌───────────┐  │                   │  ┌───────────┐  │
            │  │  clients  │  │                   │  │ WS Stream │  │
            │  │ HashMap   │  │                   │  │  split()  │  │
            │  └───────────┘  │                   │  └───────────┘  │
            │  ┌───────────┐  │                   │                 │
            │  │   rooms   │  │                   │  Read Task:     │
            │  │ HashMap   │  │                   │  WS → Command   │
            │  └───────────┘  │                   │                 │
            │  ┌───────────┐  │                   │  Write Task:    │
            │  │client_rooms│ │─────────────────>│  Server → WS    │
            │  │ HashMap   │  │  ServerMessage    │                 │
            │  └───────────┘  │      mpsc         │                 │
            └─────────────────┘                   └─────────────────┘
                     │
                     │ uses
      ┌──────────────┼──────────────┬──────────────┐
      │              │              │              │
      ▼              ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
│ types.rs │  │client.rs │  │ room.rs  │  │message.rs│
│          │  │          │  │          │  │          │
│ ClientId │  │ Client { │  │ Room {   │  │ Client/  │
│ RoomCode │  │   id,    │  │   code,  │  │ Server   │
│          │  │   user,  │  │   host,  │  │ Message  │
│          │  │   sender │  │   guest  │  │          │
│          │  │ }        │  │ }        │  │ ErrorCode│
└──────────┘  └──────────┘  └──────────┘  └──────────┘
      │              │              │              │
      └──────────────┴──────────────┴──────────────┘
                            │
                            ▼
                     ┌──────────┐
                     │ error.rs │
                     │          │
                     │ AppError │
                     │SendError │
                     └──────────┘
```

---

## 3. Actor Pattern Details

### 3.1 Why Actor Pattern?

| Pattern | Pros | Cons |
|---------|------|------|
| Arc<Mutex<T>> | Intuitive, simple | Lock contention, deadlock risk |
| Arc<RwLock<T>> | Good read performance | Still lock-based |
| **Actor (mpsc)** | No locks, message-based, scalable | Slightly complex |

### 3.2 Actor Structure

```
┌─────────────────────────────────────────────────────────────┐
│                      ChatServer Actor                       │
│  ┌───────────────────────────────────────────────────────┐ │
│  │  clients: HashMap<ClientId, Client>                   │ │
│  │  rooms: HashMap<RoomCode, Room>                       │ │
│  │  client_rooms: HashMap<ClientId, RoomCode>            │ │
│  └───────────────────────────────────────────────────────┘ │
│                           ▲                                 │
│                           │ ServerCommand (mpsc)            │
└───────────────────────────┼─────────────────────────────────┘
                            │
         ┌──────────────────┼──────────────────┐
         │                  │                  │
    ┌────▼─────┐       ┌────▼─────┐       ┌────▼─────┐
    │ Client A │       │ Client B │       │ Client C │
    │  Task    │       │  Task    │       │  Task    │
    └──────────┘       └──────────┘       └──────────┘
```

### 3.3 ServerCommand Definition

```rust
pub enum ServerCommand {
    Connect {
        client_id: ClientId,
        sender: mpsc::Sender<ServerMessage>,
    },
    Disconnect {
        client_id: ClientId,
    },
    SetUsername {
        client_id: ClientId,
        username: String,
    },
    CreateRoom {
        client_id: ClientId,
    },
    JoinRoom {
        client_id: ClientId,
        room_code: String,
    },
    Chat {
        client_id: ClientId,
        content: String,
    },
    Typing {
        client_id: ClientId,
    },
    StopTyping {
        client_id: ClientId,
    },
    LeaveRoom {
        client_id: ClientId,
    },
}
```

---

## 4. Module Dependencies

### 4.1 Dependency Graph

```
                              ┌─────────────┐
                              │   main.rs   │
                              │ (entry pt)  │
                              └──────┬──────┘
                                     │
                    ┌────────────────┼────────────────┐
                    │                │                │
                    ▼                ▼                ▼
            ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
            │  server.rs   │ │  handler.rs  │ │   lib.rs     │
            │ (ChatServer) │ │ (WS Handler) │ │  (re-export) │
            └──────┬───────┘ └──────┬───────┘ └──────────────┘
                   │                │
       ┌───────────┼────────┐       │
       │           │        │       │
       ▼           ▼        ▼       ▼
┌──────────┐ ┌──────────┐ ┌──────────┐
│ client.rs│ │ room.rs  │ │message.rs│
│ (Client) │ │  (Room)  │ │(Messages)│
└────┬─────┘ └────┬─────┘ └────┬─────┘
     │            │            │
     └────────────┼────────────┘
                  ▼
           ┌──────────┐
           │ types.rs │
           │(ClientId,│
           │RoomCode) │
           └────┬─────┘
                │
                ▼
           ┌──────────┐
           │ error.rs │
           │(AppError)│
           └──────────┘
```

### 4.2 Dependency Matrix

| Module | Dependencies |
|--------|--------------|
| `main.rs` | server, handler, tokio, tokio-tungstenite |
| `server.rs` | types, client, room, message, error |
| `handler.rs` | types, message, server (ServerCommand), tokio-tungstenite |
| `client.rs` | types, message |
| `room.rs` | types |
| `message.rs` | types, error (ErrorCode), serde |
| `types.rs` | uuid, rand |
| `error.rs` | thiserror |

---

## 5. Learning Points

| Topic | What You'll Learn |
|-------|-------------------|
| **tokio runtime** | async/await, spawn, select! |
| **Channel communication** | mpsc, oneshot patterns |
| **Actor pattern** | Message-based state management |
| **WebSocket** | Handshake, frames, split() |
| **Serde** | JSON serialization/deserialization |
| **Error handling** | Result, thiserror |
