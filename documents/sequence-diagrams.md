# Sequence Diagrams - Data Flow Sequences

---

## 1. Connection and Room Creation Flow

```
┌──────────┐          ┌───────────┐          ┌────────────┐
│ Client A │          │  Handler  │          │ ChatServer │
│  (WS)    │          │  (Task)   │          │  (Actor)   │
└────┬─────┘          └─────┬─────┘          └──────┬─────┘
     │                      │                       │
     │  TCP Connect         │                       │
     │─────────────────────>│                       │
     │                      │                       │
     │  WS Handshake        │                       │
     │<────────────────────>│                       │
     │                      │                       │
     │                      │  ServerCommand::Connect
     │                      │──────────────────────>│
     │                      │                       │
     │                      │   (register client)   │
     │                      │   clients.insert(A)   │
     │                      │                       │
     │  Connected           │                       │
     │  { client_id }       │<──────────────────────│
     │<─────────────────────│                       │
     │                      │                       │
     │  SetUsername         │                       │
     │  { "Alice" }         │                       │
     │─────────────────────>│                       │
     │                      │  ServerCommand::SetUsername
     │                      │──────────────────────>│
     │                      │                       │
     │                      │   clients[A].username │
     │                      │   = Some("Alice")     │
     │                      │                       │
     │  UsernameSet         │                       │
     │<─────────────────────│<──────────────────────│
     │                      │                       │
     │  CreateRoom          │                       │
     │─────────────────────>│                       │
     │                      │  ServerCommand::CreateRoom
     │                      │──────────────────────>│
     │                      │                       │
     │                      │   code = RoomCode::generate()
     │                      │   rooms.insert(code, Room::new(A))
     │                      │   client_rooms.insert(A, code)
     │                      │                       │
     │  RoomCreated         │                       │
     │  { "ABC123" }        │<──────────────────────│
     │<─────────────────────│                       │
     │                      │                       │
```

---

## 2. Partner Joining Flow

```
┌──────────┐          ┌──────────┐          ┌────────────┐          ┌──────────┐
│ Client B │          │Handler B │          │ ChatServer │          │Handler A │
└────┬─────┘          └────┬─────┘          └──────┬─────┘          └────┬─────┘
     │                     │                       │                     │
     │  (connection, SetUsername omitted)          │                     │
     │                     │                       │                     │
     │  JoinRoom           │                       │                     │
     │  { "ABC123" }       │                       │                     │
     │────────────────────>│                       │                     │
     │                     │  ServerCommand::JoinRoom                    │
     │                     │──────────────────────>│                     │
     │                     │                       │                     │
     │                     │   room = rooms.get("ABC123")                │
     │                     │   room.guest = Some(B)                      │
     │                     │   client_rooms.insert(B, "ABC123")          │
     │                     │                       │                     │
     │                     │                       │   PartnerJoined     │
     │                     │                       │   { "Bob" }         │
     │                     │                       │────────────────────>│
     │                     │                       │                     │
     │  RoomJoined         │                       │                     │──> Client A
     │  { partner: "Alice" }                       │                     │
     │<────────────────────│<──────────────────────│                     │
     │                     │                       │                     │
```

---

## 3. Chat and Typing Flow

```
┌──────────┐          ┌────────────┐          ┌──────────┐
│ Client A │          │ ChatServer │          │ Client B │
└────┬─────┘          └──────┬─────┘          └────┬─────┘
     │                       │                     │
     │  Typing               │                     │
     │──────────────────────>│                     │
     │                       │                     │
     │                       │  clients[A].is_typing = true
     │                       │  partner = room.get_partner(A) → B
     │                       │                     │
     │                       │  PartnerTyping      │
     │                       │────────────────────>│
     │                       │                     │
     │  Chat                 │                     │
     │  { "Hello!" }         │                     │
     │──────────────────────>│                     │
     │                       │                     │
     │                       │  clients[A].is_typing = false
     │                       │                     │
     │                       │  PartnerStopTyping  │
     │                       │────────────────────>│
     │                       │                     │
     │                       │  Chat               │
     │                       │  { from: "Alice",   │
     │                       │    content: "Hello!"}
     │                       │────────────────────>│
     │                       │                     │
     │                       │                     │
     │                       │  Typing             │
     │                       │<────────────────────│
     │                       │                     │
     │  PartnerTyping        │                     │
     │<──────────────────────│                     │
     │                       │                     │
```

---

## 4. Disconnection Flow

```
┌──────────┐          ┌───────────┐          ┌────────────┐          ┌───────────┐
│ Client A │          │ Handler A │          │ ChatServer │          │ Handler B │
└────┬─────┘          └─────┬─────┘          └──────┬─────┘          └─────┬─────┘
     │                      │                       │                      │
     │  [Connection Lost]   │                       │                      │
     X─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─>│                       │                      │
                            │                       │                      │
                            │  ServerCommand::Disconnect                   │
                            │──────────────────────>│                      │
                            │                       │                      │
                            │   room_code = client_rooms.get(A)            │
                            │   room = rooms.get(room_code)                │
                            │   partner = room.get_partner(A) → B          │
                            │                       │                      │
                            │   room.remove_client(A)                      │
                            │   clients.remove(A)                          │
                            │   client_rooms.remove(A)                     │
                            │                       │                      │
                            │                       │   PartnerLeft        │
                            │                       │─────────────────────>│
                            │                       │                      │
                            │                       │                      │──> Client B
                            │                       │                      │
```

---

## 5. Voluntary Leave Room Flow

```
┌──────────┐          ┌───────────┐          ┌────────────┐          ┌───────────┐
│ Client A │          │ Handler A │          │ ChatServer │          │ Handler B │
└────┬─────┘          └─────┬─────┘          └──────┬─────┘          └─────┬─────┘
     │                      │                       │                      │
     │  LeaveRoom           │                       │                      │
     │─────────────────────>│                       │                      │
     │                      │  ServerCommand::LeaveRoom                    │
     │                      │──────────────────────>│                      │
     │                      │                       │                      │
     │                      │   room_code = client_rooms.get(A)            │
     │                      │   room = rooms.get(room_code)                │
     │                      │   partner = room.get_partner(A) → B          │
     │                      │                       │                      │
     │                      │   should_delete = room.remove_client(A)      │
     │                      │   client_rooms.remove(A)                     │
     │                      │                       │                      │
     │                      │   if should_delete:                          │
     │                      │     rooms.remove(room_code)                  │
     │                      │                       │                      │
     │                      │                       │   PartnerLeft        │
     │                      │                       │─────────────────────>│
     │                      │                       │                      │
     │                      │                       │                      │──> Client B
     │                      │                       │                      │
     │  (connection kept)   │                       │                      │
     │  (can CreateRoom/    │                       │                      │
     │   JoinRoom again)    │                       │                      │
     │                      │                       │                      │
```

---

## 6. Error Flow

### 6.1 Room Not Found

```
┌──────────┐          ┌───────────┐          ┌────────────┐
│ Client   │          │  Handler  │          │ ChatServer │
└────┬─────┘          └─────┬─────┘          └──────┬─────┘
     │                      │                       │
     │  JoinRoom            │                       │
     │  { "INVALID" }       │                       │
     │─────────────────────>│                       │
     │                      │  ServerCommand::JoinRoom
     │                      │──────────────────────>│
     │                      │                       │
     │                      │   rooms.get("INVALID") → None
     │                      │                       │
     │  Error               │                       │
     │  { code: "room_not_found",                   │
     │    message: "..." }  │<──────────────────────│
     │<─────────────────────│                       │
     │                      │                       │
```

### 6.2 Username Not Set

```
┌──────────┐          ┌───────────┐          ┌────────────┐
│ Client   │          │  Handler  │          │ ChatServer │
└────┬─────┘          └─────┬─────┘          └──────┬─────┘
     │                      │                       │
     │  CreateRoom          │                       │
     │  (without username)  │                       │
     │─────────────────────>│                       │
     │                      │  ServerCommand::CreateRoom
     │                      │──────────────────────>│
     │                      │                       │
     │                      │   clients[id].username → None
     │                      │                       │
     │  Error               │                       │
     │  { code: "username_required",               │
     │    message: "..." }  │<──────────────────────│
     │<─────────────────────│                       │
     │                      │                       │
```

---

## 7. Internal Data Flow

### Handler → ChatServer (Command Transmission)

```
┌─────────────────────────────────────────────────────────────┐
│                      Handler Task                           │
│                                                             │
│  1. Receive Text frame from WebSocket                       │
│  2. Parse JSON → ClientMessage                              │
│  3. Convert ClientMessage → ServerCommand                   │
│  4. cmd_tx.send(command).await                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ mpsc::Sender<ServerCommand>
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                     ChatServer Actor                        │
│                                                             │
│  loop {                                                     │
│      if let Some(cmd) = receiver.recv().await {             │
│          self.handle_command(cmd).await;                    │
│      }                                                      │
│  }                                                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### ChatServer → Handler (Response Transmission)

```
┌─────────────────────────────────────────────────────────────┐
│                     ChatServer Actor                        │
│                                                             │
│  // Send message to client                                  │
│  let client = self.clients.get(&client_id)?;                │
│  client.sender.send(ServerMessage::Chat { ... }).await;     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ mpsc::Sender<ServerMessage>
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      Handler Task                           │
│                                                             │
│  // Receive in Write loop                                   │
│  while let Some(msg) = msg_rx.recv().await {                │
│      let json = serde_json::to_string(&msg)?;               │
│      ws_sender.send(Message::Text(json)).await?;            │
│  }                                                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```
