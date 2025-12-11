# Sequence Diagrams - 데이터 흐름 시퀀스

---

## 1. 연결 및 방 생성 흐름

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
     │                      │   (client 등록)        │
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

## 2. 상대방 입장 흐름

```
┌──────────┐          ┌──────────┐          ┌────────────┐          ┌──────────┐
│ Client B │          │Handler B │          │ ChatServer │          │Handler A │
└────┬─────┘          └────┬─────┘          └──────┬─────┘          └────┬─────┘
     │                     │                       │                     │
     │  (연결, SetUsername 생략)                    │                     │
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

## 3. 채팅 및 타이핑 흐름

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

## 4. 연결 해제 흐름

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

## 5. 방 나가기 흐름 (자발적 퇴장)

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
     │  (연결은 유지됨)       │                       │                      │
     │  (다시 CreateRoom/   │                       │                      │
     │   JoinRoom 가능)     │                       │                      │
     │                      │                       │                      │
```

---

## 6. 에러 흐름

### 6.1 방을 찾을 수 없음

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

### 6.2 사용자명 미설정

```
┌──────────┐          ┌───────────┐          ┌────────────┐
│ Client   │          │  Handler  │          │ ChatServer │
└────┬─────┘          └─────┬─────┘          └──────┬─────┘
     │                      │                       │
     │  CreateRoom          │                       │
     │  (username 없이)      │                       │
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

## 7. 내부 데이터 흐름

### Handler → ChatServer (명령 전송)

```
┌─────────────────────────────────────────────────────────────┐
│                      Handler Task                           │
│                                                             │
│  1. WebSocket에서 Text 프레임 수신                           │
│  2. JSON 파싱 → ClientMessage                               │
│  3. ClientMessage → ServerCommand 변환                      │
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

### ChatServer → Handler (응답 전송)

```
┌─────────────────────────────────────────────────────────────┐
│                     ChatServer Actor                        │
│                                                             │
│  // 클라이언트에게 메시지 전송                                │
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
│  // Write 루프에서 수신                                      │
│  while let Some(msg) = msg_rx.recv().await {                │
│      let json = serde_json::to_string(&msg)?;               │
│      ws_sender.send(Message::Text(json)).await?;            │
│  }                                                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```
