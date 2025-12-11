# Error Handling - 에러 처리 전략

---

## 1. 에러 타입 정의 (error.rs)

```rust
use thiserror::Error;

/// 애플리케이션 레벨 에러
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

/// 메시지 전송 에러
#[derive(Debug, Error)]
pub enum SendError {
    #[error("Channel closed")]
    ChannelClosed,
}
```

---

## 2. 에러 분류

### 2.1 치명적 에러 (연결 종료)

연결을 유지할 수 없는 에러:

| 에러 | 설명 | 처리 |
|------|------|------|
| `WebSocket` | WS 프로토콜 에러 | 연결 종료 |
| `Io` | 네트워크 에러 | 연결 종료 |
| `ChannelSend` | 내부 채널 끊김 | 연결 종료 |

### 2.2 비즈니스 에러 (에러 메시지 전송)

클라이언트에게 알려주고 계속 진행:

| 에러 | 설명 | ErrorCode |
|------|------|-----------|
| `RoomNotFound` | 방 코드 없음 | `room_not_found` |
| `RoomFull` | 방 정원 초과 | `room_full` |
| `UsernameRequired` | 사용자명 필요 | `username_required` |
| `NotInRoom` | 방 미참여 | `not_in_room` |
| `AlreadyInRoom` | 이미 방에 있음 | `already_in_room` |
| `Json` | JSON 파싱 실패 | `invalid_message` |

---

## 3. 에러 처리 흐름

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
│  1. Disconnect     │              │                         │
│     명령 전송       │              │  ServerMessage::Error   │
│  2. 리소스 정리     │              │  { code, message }      │
│  3. Handler 종료   │              │                         │
└────────────────────┘              └────────────┬────────────┘
                                                 │
                                                 ▼
                                    ┌─────────────────────────┐
                                    │  Continue Normal        │
                                    │  Operation              │
                                    │                         │
                                    │  (연결 유지)             │
                                    └─────────────────────────┘
```

---

## 4. 에러 → ServerMessage 변환

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
            // 치명적 에러는 변환하지 않음 (연결 종료됨)
            _ => {
                (ErrorCode::InvalidMessage, "Internal error".to_string())
            }
        };
        ServerMessage::Error { code, message }
    }
}
```

---

## 5. Handler에서의 에러 처리 패턴

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

    // 서버에 연결 등록
    cmd_tx.send(ServerCommand::Connect {
        client_id,
        sender: msg_tx,
    }).await.map_err(|_| AppError::ChannelSend)?;

    // 연결 성공 메시지 전송
    let connected_msg = serde_json::to_string(&ServerMessage::Connected {
        client_id: client_id.to_string(),
    })?;
    ws_sender.send(Message::Text(connected_msg)).await?;

    // Read/Write 분리
    let read_handle = tokio::spawn(async move {
        while let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    // JSON 파싱 에러는 비즈니스 에러로 처리
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(client_msg) => {
                            let cmd = client_message_to_command(client_id, client_msg);
                            if cmd_tx.send(cmd).await.is_err() {
                                break; // 서버 종료
                            }
                        }
                        Err(e) => {
                            // 파싱 에러: 에러 메시지 전송 후 계속
                            let err_msg = ServerMessage::Error {
                                code: ErrorCode::InvalidMessage,
                                message: format!("Invalid JSON: {}", e),
                            };
                            // msg_tx로 에러 전송 (별도 처리 필요)
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break, // 치명적 에러
                _ => {} // Ping/Pong은 라이브러리가 처리
            }
        }
    });

    // Write 루프
    let write_handle = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(_) => continue, // 직렬화 실패 (거의 없음)
            };
            if ws_sender.send(Message::Text(json)).await.is_err() {
                break; // 연결 끊김
            }
        }
    });

    // 둘 중 하나라도 종료되면 정리
    tokio::select! {
        _ = read_handle => {}
        _ = write_handle => {}
    }

    // 연결 해제 명령
    let _ = cmd_tx.send(ServerCommand::Disconnect { client_id }).await;

    Ok(())
}
```

---

## 6. ChatServer에서의 에러 처리 패턴

```rust
// server.rs

impl ChatServer {
    async fn handle_create_room(&mut self, client_id: ClientId) {
        // 클라이언트 존재 확인
        let client = match self.clients.get(&client_id) {
            Some(c) => c,
            None => return, // 클라이언트 없음 (비정상 상황)
        };

        // 사용자명 확인
        if client.username.is_none() {
            let _ = client.send(AppError::UsernameRequired.into()).await;
            return;
        }

        // 이미 방에 있는지 확인
        if self.client_rooms.contains_key(&client_id) {
            let _ = client.send(AppError::AlreadyInRoom.into()).await;
            return;
        }

        // 방 생성
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

        // 사용자명 확인
        if client.username.is_none() {
            let _ = client.send(AppError::UsernameRequired.into()).await;
            return;
        }

        // 이미 방에 있는지 확인
        if self.client_rooms.contains_key(&client_id) {
            let _ = client.send(AppError::AlreadyInRoom.into()).await;
            return;
        }

        let room_code = RoomCode::from_string(room_code);

        // 방 존재 확인
        let room = match self.rooms.get_mut(&room_code) {
            Some(r) => r,
            None => {
                let _ = client.send(AppError::RoomNotFound(room_code.to_string()).into()).await;
                return;
            }
        };

        // 방 정원 확인
        if room.is_full() {
            let _ = client.send(AppError::RoomFull.into()).await;
            return;
        }

        // 입장 처리
        let host_id = room.host;
        room.guest = Some(client_id);
        self.client_rooms.insert(client_id, room_code.clone());

        // 호스트 이름 조회
        let host_name = self.clients.get(&host_id)
            .and_then(|c| c.username.clone());

        // 입장자에게 알림
        let _ = client.send(ServerMessage::RoomJoined {
            room_code: room_code.to_string(),
            partner: host_name,
        }).await;

        // 호스트에게 알림
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

## 7. Result 체이닝 예시

```rust
/// 상대방에게 메시지 전송하는 헬퍼 함수
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

## 8. 에러 로깅

```rust
use tracing::{info, warn, error};

// 치명적 에러
error!("WebSocket error for client {}: {}", client_id, err);

// 비즈니스 에러
warn!("Client {} tried to join non-existent room: {}", client_id, room_code);

// 정상 동작
info!("Client {} created room {}", client_id, room_code);
```
