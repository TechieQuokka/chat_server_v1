# Data Structures - 데이터 구조 설계

---

## 1. 타입 정의 (types.rs)

### 1.1 ClientId

클라이언트 고유 식별자. Newtype 패턴으로 타입 안전성 확보.

```rust
use uuid::Uuid;

/// 클라이언트 고유 식별자 (newtype 패턴)
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

방 코드 (6자리 대문자 영숫자).

```rust
/// 방 코드 (6자리 대문자 영숫자)
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

## 2. 클라이언트 구조체 (client.rs)

```rust
use crate::types::ClientId;
use crate::message::ServerMessage;
use crate::error::SendError;
use tokio::sync::mpsc;

/// 연결된 클라이언트 정보
#[derive(Debug)]
pub struct Client {
    /// 고유 식별자
    pub id: ClientId,
    /// 사용자 이름 (설정 전 None)
    pub username: Option<String>,
    /// 서버 → 클라이언트 메시지 전송 채널
    pub sender: mpsc::Sender<ServerMessage>,
    /// 현재 타이핑 중인지 여부
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

    /// 클라이언트에게 메시지 전송
    pub async fn send(&self, msg: ServerMessage) -> Result<(), SendError> {
        self.sender
            .send(msg)
            .await
            .map_err(|_| SendError::ChannelClosed)
    }

    /// 사용자명 반환 (없으면 "Unknown")
    pub fn display_name(&self) -> &str {
        self.username.as_deref().unwrap_or("Unknown")
    }
}
```

---

## 3. 방 구조체 (room.rs)

```rust
use crate::types::{ClientId, RoomCode};
use std::time::Instant;

/// 1:1 채팅방
#[derive(Debug)]
pub struct Room {
    /// 방 코드
    pub code: RoomCode,
    /// 방 생성자 (호스트)
    pub host: ClientId,
    /// 입장한 상대방 (게스트)
    pub guest: Option<ClientId>,
    /// 방 생성 시간
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

    /// 방이 꽉 찼는지 확인 (2명)
    pub fn is_full(&self) -> bool {
        self.guest.is_some()
    }

    /// 방이 비어있는지 확인 (호스트만 있음)
    pub fn is_empty(&self) -> bool {
        self.guest.is_none()
    }

    /// 상대방 ClientId 반환
    pub fn get_partner(&self, client_id: ClientId) -> Option<ClientId> {
        if self.host == client_id {
            self.guest
        } else if self.guest == Some(client_id) {
            Some(self.host)
        } else {
            None
        }
    }

    /// 특정 클라이언트가 방에 있는지 확인
    pub fn contains(&self, client_id: ClientId) -> bool {
        self.host == client_id || self.guest == Some(client_id)
    }

    /// 클라이언트 제거 (퇴장 처리)
    /// 반환값: 방을 삭제해야 하는지 여부
    pub fn remove_client(&mut self, client_id: ClientId) -> bool {
        if self.host == client_id {
            // 호스트가 나가면 게스트를 호스트로 승격
            if let Some(guest) = self.guest.take() {
                self.host = guest;
                false // 방 유지
            } else {
                true // 방 삭제
            }
        } else if self.guest == Some(client_id) {
            self.guest = None;
            false // 방 유지
        } else {
            false
        }
    }
}
```

---

## 4. 상태 관계도

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
  │                           guest: None,      ◄─── 대기 중         │
  │                           created_at: Instant                   │
  │                         }                                       │
  │                                                                 │
  └─────────────────────────────────────────────────────────────────┘

  client_rooms: HashMap<ClientId, RoomCode>  (빠른 조회용)
  ┌─────────────────────────────────────────────────────────────────┐
  │                                                                 │
  │  ClientId(uuid1) ──► RoomCode("ABC123")                         │
  │  ClientId(uuid2) ──► RoomCode("ABC123")                         │
  │  ClientId(uuid3) ──► RoomCode("XYZ789")                         │
  │                                                                 │
  └─────────────────────────────────────────────────────────────────┘
```

---

## 5. Newtype 패턴의 이점

### 타입 안전성
```rust
// Bad: 실수로 잘못된 타입 사용 가능
fn join_room(client_id: Uuid, room_code: String) { ... }

// Good: 컴파일 타임에 타입 체크
fn join_room(client_id: ClientId, room_code: RoomCode) { ... }
```

### 메서드 캡슐화
```rust
// RoomCode만의 메서드
impl RoomCode {
    pub fn generate() -> Self { ... }
    pub fn is_valid(&self) -> bool { ... }
}
```

### Hash/Eq 자동 구현
```rust
#[derive(Hash, Eq, PartialEq)]
pub struct ClientId(pub Uuid);

// HashMap의 키로 바로 사용 가능
let clients: HashMap<ClientId, Client> = HashMap::new();
```
