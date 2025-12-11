//! Message protocol definitions
//!
//! JSON-based bidirectional message protocol using Serde's tagged enum
//! for type-safe serialization/deserialization.

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Client → Server message
///
/// All messages from client to server. Uses tagged enum with snake_case naming.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Set username (required before room operations)
    SetUsername { username: String },
    /// Create a new room
    CreateRoom,
    /// Join an existing room by code
    JoinRoom { room_code: String },
    /// Send a chat message
    Chat { content: String },
    /// Indicate typing started
    Typing,
    /// Indicate typing stopped
    StopTyping,
    /// Leave the current room
    LeaveRoom,
}

/// Server → Client message
///
/// All messages from server to client. Uses tagged enum with snake_case naming.
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
    /// Partner joined the room
    PartnerJoined { username: String },
    /// Chat message received
    Chat { from: String, content: String },
    /// Partner is typing
    PartnerTyping,
    /// Partner stopped typing
    PartnerStopTyping,
    /// Partner left the room
    PartnerLeft,
    /// Error occurred
    Error { code: ErrorCode, message: String },
}

/// Error codes for ServerMessage::Error
///
/// Represents different error scenarios that can be communicated to clients.
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
    /// Already in a room
    AlreadyInRoom,
    /// Invalid message format
    InvalidMessage,
}

/// Convert AppError to ServerMessage for client notification
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
            // Fatal errors are not typically converted (connection closes)
            _ => {
                (ErrorCode::InvalidMessage, "Internal error".to_string())
            }
        };
        ServerMessage::Error { code, message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_deserialize() {
        let json = r#"{"type": "set_username", "username": "Alice"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::SetUsername { username } => assert_eq!(username, "Alice"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_server_message_serialize() {
        let msg = ServerMessage::Connected {
            client_id: "test-id".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"connected\""));
        assert!(json.contains("\"client_id\":\"test-id\""));
    }

    #[test]
    fn test_error_code_serialize() {
        let msg = ServerMessage::Error {
            code: ErrorCode::RoomNotFound,
            message: "Test".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"code\":\"room_not_found\""));
    }
}
