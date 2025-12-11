# Project Structure - 프로젝트 구조

---

## 1. 디렉토리 구조

```
chat_server_v1/
├── Cargo.toml              # 프로젝트 메타데이터 및 의존성
├── Cargo.lock              # 의존성 버전 잠금
├── documents/              # 설계 문서
│   ├── architecture.md
│   ├── data-structures.md
│   ├── message-protocol.md
│   ├── sequence-diagrams.md
│   ├── error-handling.md
│   └── project-structure.md (이 파일)
└── src/
    ├── main.rs             # 진입점: TCP Listener, 서버 시작
    ├── lib.rs              # 모듈 선언 및 re-export
    ├── types.rs            # ClientId, RoomCode (newtype)
    ├── message.rs          # ClientMessage, ServerMessage, ErrorCode
    ├── client.rs           # Client 구조체
    ├── room.rs             # Room 구조체
    ├── server.rs           # ChatServer Actor, ServerCommand
    ├── handler.rs          # WebSocket 연결 핸들러
    └── error.rs            # AppError, SendError
```

---

## 2. Cargo.toml

```toml
[package]
name = "chat_server_v1"
version = "0.1.0"
edition = "2021"
authors = ["Your Name"]
description = "A simple 1:1 WebSocket chat server using tokio-tungstenite"

[dependencies]
# Async runtime
tokio = { version = "1.41", features = ["full"] }

# WebSocket
tokio-tungstenite = "0.24"

# Futures utilities (StreamExt, SinkExt)
futures-util = { version = "0.3", default-features = false, features = ["sink", "std"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# UUID generation
uuid = { version = "1.11", features = ["v4"] }

# Random generation (for room code)
rand = "0.8"

# Error handling
thiserror = "2.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
lto = true
codegen-units = 1
```

---

## 3. 모듈별 책임

### 3.1 main.rs
- 애플리케이션 진입점
- tracing 초기화
- TcpListener 바인딩
- ChatServer Actor 시작
- 연결 수락 루프

```rust
// 주요 흐름
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 로깅 초기화
    tracing_subscriber::fmt()
        .with_env_filter("chat_server_v1=debug,info")
        .init();

    // 2. TCP 리스너 시작
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    // 3. ChatServer Actor 채널 생성 및 시작
    let (cmd_tx, cmd_rx) = mpsc::channel(256);
    let server = ChatServer::new(cmd_rx);
    tokio::spawn(server.run());

    // 4. 연결 수락 루프
    loop {
        let (stream, _) = listener.accept().await?;
        let cmd_tx = cmd_tx.clone();
        tokio::spawn(handle_connection(stream, cmd_tx));
    }
}
```

### 3.2 lib.rs
- 모듈 선언
- 주요 타입 re-export

```rust
pub mod types;
pub mod message;
pub mod client;
pub mod room;
pub mod server;
pub mod handler;
pub mod error;

pub use types::{ClientId, RoomCode};
pub use message::{ClientMessage, ServerMessage, ErrorCode};
pub use client::Client;
pub use room::Room;
pub use server::{ChatServer, ServerCommand};
pub use error::{AppError, SendError};
```

### 3.3 types.rs
- 기본 타입 정의
- `ClientId`: UUID 기반 클라이언트 식별자
- `RoomCode`: 6자리 영숫자 방 코드

### 3.4 message.rs
- 프로토콜 메시지 정의
- `ClientMessage`: 클라이언트 → 서버
- `ServerMessage`: 서버 → 클라이언트
- `ErrorCode`: 에러 코드 enum

### 3.5 client.rs
- `Client` 구조체
- 클라이언트 상태 (id, username, sender, is_typing)
- 메시지 전송 헬퍼 메서드

### 3.6 room.rs
- `Room` 구조체
- 방 상태 (code, host, guest, created_at)
- 방 관리 메서드 (is_full, get_partner, remove_client)

### 3.7 server.rs
- `ChatServer` Actor
- `ServerCommand` enum
- 상태 관리 (clients, rooms, client_rooms)
- 명령 처리 핸들러

### 3.8 handler.rs
- WebSocket 연결 핸들러
- WS 핸드셰이크
- Read/Write 태스크 분리
- ClientMessage → ServerCommand 변환

### 3.9 error.rs
- `AppError`: 애플리케이션 에러
- `SendError`: 채널 전송 에러
- 에러 → ServerMessage 변환

---

## 4. 데이터 흐름 요약

```
                    ┌─────────────────────────────────┐
                    │           main.rs               │
                    │   TcpListener.accept()          │
                    └───────────────┬─────────────────┘
                                    │
                                    │ TcpStream
                                    ▼
                    ┌─────────────────────────────────┐
                    │          handler.rs             │
                    │                                 │
                    │  ┌─────────────────────────┐    │
                    │  │ WebSocket Handshake     │    │
                    │  └─────────────────────────┘    │
                    │                                 │
                    │  ┌─────────────────────────┐    │
                    │  │ Read Task               │    │
                    │  │ WS → ClientMessage      │    │
                    │  │ → ServerCommand         │───────────┐
                    │  └─────────────────────────┘    │       │
                    │                                 │       │
                    │  ┌─────────────────────────┐    │       │
                    │  │ Write Task              │◄───────────┼──┐
                    │  │ ServerMessage → WS      │    │       │  │
                    │  └─────────────────────────┘    │       │  │
                    └─────────────────────────────────┘       │  │
                                                              │  │
                              mpsc::Sender<ServerCommand>     │  │
                                                              │  │
                    ┌─────────────────────────────────────────▼──┤
                    │              server.rs                     │
                    │            ChatServer Actor                │
                    │                                            │
                    │  ┌────────────────────────────────────┐    │
                    │  │ clients: HashMap<ClientId, Client> │    │
                    │  │ rooms: HashMap<RoomCode, Room>     │    │
                    │  │ client_rooms: HashMap<ClientId,    │    │
                    │  │               RoomCode>            │    │
                    │  └────────────────────────────────────┘    │
                    │                                            │
                    │  handle_command() → ServerMessage ─────────┘
                    │                                            │
                    └────────────────────────────────────────────┘
                                        │
                                        │ uses
                    ┌───────────────────┼───────────────────┐
                    │                   │                   │
                    ▼                   ▼                   ▼
            ┌───────────────┐   ┌───────────────┐   ┌───────────────┐
            │   client.rs   │   │   room.rs     │   │  message.rs   │
            │               │   │               │   │               │
            │   Client      │   │   Room        │   │ ClientMessage │
            │               │   │               │   │ ServerMessage │
            └───────────────┘   └───────────────┘   └───────────────┘
                    │                   │                   │
                    └───────────────────┼───────────────────┘
                                        │
                                        ▼
                                ┌───────────────┐
                                │   types.rs    │
                                │   error.rs    │
                                └───────────────┘
```

---

## 5. 빌드 및 실행

```bash
# 개발 빌드
cargo build

# 릴리즈 빌드
cargo build --release

# 실행
cargo run

# 로그 레벨 설정
RUST_LOG=debug cargo run
RUST_LOG=chat_server_v1=trace cargo run
```

---

## 6. 테스트 (향후 확장)

```
src/
└── tests/
    ├── integration_test.rs   # 통합 테스트
    └── unit/
        ├── room_test.rs      # Room 단위 테스트
        └── message_test.rs   # 메시지 직렬화 테스트
```

```bash
# 테스트 실행
cargo test

# 특정 테스트만
cargo test room_test
```
