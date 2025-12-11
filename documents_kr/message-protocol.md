# Message Protocol - 메시지 프로토콜

---

## 1. 개요

JSON 기반 양방향 메시지 프로토콜. Serde의 tagged enum을 사용하여 타입 안전하게 직렬화/역직렬화.

---

## 2. 클라이언트 → 서버 메시지 (ClientMessage)

```rust
use serde::{Deserialize, Serialize};

/// 클라이언트 → 서버 메시지
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// 사용자명 설정
    SetUsername { username: String },
    /// 새 방 생성
    CreateRoom,
    /// 기존 방 입장
    JoinRoom { room_code: String },
    /// 채팅 메시지 전송
    Chat { content: String },
    /// 타이핑 시작
    Typing,
    /// 타이핑 중지
    StopTyping,
    /// 방 나가기
    LeaveRoom,
}
```

### JSON 예시

```json
// 사용자명 설정
{ "type": "set_username", "username": "Alice" }

// 방 생성
{ "type": "create_room" }

// 방 입장
{ "type": "join_room", "room_code": "ABC123" }

// 채팅 메시지
{ "type": "chat", "content": "Hello!" }

// 타이핑 시작
{ "type": "typing" }

// 타이핑 중지
{ "type": "stop_typing" }

// 방 나가기
{ "type": "leave_room" }
```

---

## 3. 서버 → 클라이언트 메시지 (ServerMessage)

```rust
/// 서버 → 클라이언트 메시지
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// 연결 성공, 클라이언트 ID 발급
    Connected { client_id: String },
    /// 사용자명 설정 완료
    UsernameSet { username: String },
    /// 방 생성 완료
    RoomCreated { room_code: String },
    /// 방 입장 완료
    RoomJoined {
        room_code: String,
        partner: Option<String>,
    },
    /// 상대방 입장
    PartnerJoined { username: String },
    /// 채팅 메시지 수신
    Chat { from: String, content: String },
    /// 상대방 타이핑 중
    PartnerTyping,
    /// 상대방 타이핑 중지
    PartnerStopTyping,
    /// 상대방 퇴장
    PartnerLeft,
    /// 에러
    Error { code: ErrorCode, message: String },
}
```

### JSON 예시

```json
// 연결 성공
{ "type": "connected", "client_id": "550e8400-e29b-41d4-a716-446655440000" }

// 사용자명 설정 완료
{ "type": "username_set", "username": "Alice" }

// 방 생성 완료
{ "type": "room_created", "room_code": "ABC123" }

// 방 입장 완료 (상대방 있음)
{ "type": "room_joined", "room_code": "ABC123", "partner": "Alice" }

// 방 입장 완료 (상대방 없음, 대기 중)
{ "type": "room_joined", "room_code": "ABC123", "partner": null }

// 상대방 입장
{ "type": "partner_joined", "username": "Bob" }

// 채팅 메시지 수신
{ "type": "chat", "from": "Alice", "content": "Hello!" }

// 상대방 타이핑 중
{ "type": "partner_typing" }

// 상대방 타이핑 중지
{ "type": "partner_stop_typing" }

// 상대방 퇴장
{ "type": "partner_left" }

// 에러
{ "type": "error", "code": "room_not_found", "message": "Room 'XYZ999' not found" }
```

---

## 4. 에러 코드 (ErrorCode)

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// 사용자명 미설정 상태에서 작업 시도
    UsernameRequired,
    /// 존재하지 않는 방 코드
    RoomNotFound,
    /// 이미 2명이 있는 방
    RoomFull,
    /// 방에 들어가지 않은 상태에서 채팅 시도
    NotInRoom,
    /// 잘못된 메시지 형식
    InvalidMessage,
}
```

### 에러 시나리오

| 에러 코드 | 발생 상황 |
|-----------|-----------|
| `username_required` | 사용자명 설정 전에 방 생성/입장/채팅 시도 |
| `room_not_found` | 존재하지 않는 방 코드로 입장 시도 |
| `room_full` | 이미 2명이 있는 방에 입장 시도 |
| `not_in_room` | 방에 들어가지 않은 상태에서 채팅/타이핑 시도 |
| `invalid_message` | JSON 파싱 실패 또는 알 수 없는 메시지 타입 |

---

## 5. 통신 시퀀스

### 5.1 기본 연결 흐름

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

### 5.2 방 생성 및 입장

```
Client A                  Server                  Client B
  │                          │                        │
  │─── CreateRoom ──────────>│                        │
  │                          │                        │
  │<── RoomCreated ──────────│                        │
  │    { room_code: "ABC" }  │                        │
  │                          │                        │
  │    (A가 B에게 "ABC" 공유) │                        │
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

### 5.3 채팅 및 타이핑

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

### 5.4 연결 해제

```
Client A                  Server                  Client B
  │                          │                        │
  │─── [Connection Lost] ───>│                        │
  X                          │                        │
                             │───── PartnerLeft ─────>│
                             │                        │
                             │  (방에 B만 남음)         │
                             │                        │
```

---

## 6. Serde 설정 설명

### Tagged Enum
```rust
#[serde(tag = "type")]
```
- JSON에 `"type"` 필드를 추가하여 enum variant 구분
- 예: `{ "type": "chat", "content": "Hello" }`

### Rename All
```rust
#[serde(rename_all = "snake_case")]
```
- Rust의 PascalCase를 JSON의 snake_case로 변환
- 예: `PartnerTyping` → `"partner_typing"`
