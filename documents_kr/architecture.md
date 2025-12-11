# 1:1 WebSocket Chat Server - Architecture Design

> tokio-tungstenite 기반, Actor 패턴, 방 코드 방식, 타이핑 인디케이터 포함

---

## 1. 개요

### 1.1 프로젝트 목표
- 학습용 1:1 WebSocket 채팅 서버 구현
- DB 없이 인메모리 상태 관리
- Rust의 async/await 및 Actor 패턴 학습

### 1.2 기술 스택
| 구분 | 기술 |
|------|------|
| Runtime | tokio |
| WebSocket | tokio-tungstenite |
| Serialization | serde, serde_json |
| ID 생성 | uuid, rand |
| 에러 처리 | thiserror |
| 로깅 | tracing |

### 1.3 핵심 기능
- WebSocket 연결 수락 및 핸드셰이크
- 사용자명 설정
- 방 생성 (6자리 코드)
- 방 입장 (코드로 입장)
- 실시간 채팅
- 타이핑 인디케이터
- 연결 해제 처리

---

## 2. 아키텍처 개요

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

## 3. Actor 패턴 상세

### 3.1 왜 Actor 패턴인가?

| 패턴 | 장점 | 단점 |
|------|------|------|
| Arc<Mutex<T>> | 직관적, 간단 | Lock contention, 데드락 위험 |
| Arc<RwLock<T>> | 읽기 성능 좋음 | 여전히 락 기반 |
| **Actor (mpsc)** | 락 없음, 메시지 기반, 확장성 | 약간의 복잡성 |

### 3.2 Actor 구조

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

### 3.3 ServerCommand 정의

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

## 4. 모듈 의존성

### 4.1 의존성 그래프

```
                              ┌─────────────┐
                              │   main.rs   │
                              │  (진입점)    │
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

### 4.2 의존성 매트릭스

| 모듈 | 의존 대상 |
|------|-----------|
| `main.rs` | server, handler, tokio, tokio-tungstenite |
| `server.rs` | types, client, room, message, error |
| `handler.rs` | types, message, server (ServerCommand), tokio-tungstenite |
| `client.rs` | types, message |
| `room.rs` | types |
| `message.rs` | types, error (ErrorCode), serde |
| `types.rs` | uuid, rand |
| `error.rs` | thiserror |

---

## 5. 학습 포인트

| 주제 | 배울 내용 |
|------|-----------|
| **tokio 런타임** | async/await, spawn, select! |
| **채널 통신** | mpsc, oneshot 패턴 |
| **Actor 패턴** | 메시지 기반 상태 관리 |
| **WebSocket** | 핸드셰이크, 프레임, split() |
| **Serde** | JSON 직렬화/역직렬화 |
| **에러 처리** | Result, thiserror |
