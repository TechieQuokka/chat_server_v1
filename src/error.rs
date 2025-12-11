//! Error types for the chat server
//!
//! Defines application-level errors and message send errors.
//! Uses thiserror for ergonomic error definitions.

use thiserror::Error;

/// Application-level errors
///
/// Covers both fatal errors (connection termination) and
/// business errors (send error message to client).
#[derive(Debug, Error)]
pub enum AppError {
    /// WebSocket protocol error (fatal)
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// JSON serialization/deserialization error
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// IO error (fatal)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Channel send error (fatal - internal channel broken)
    #[error("Channel send error")]
    ChannelSend,

    /// Room not found with the given code
    #[error("Room not found: {0}")]
    RoomNotFound(String),

    /// Room is full (already has 2 participants)
    #[error("Room is full")]
    RoomFull,

    /// Username is required but not set
    #[error("Username required")]
    UsernameRequired,

    /// Client is not in any room
    #[error("Not in room")]
    NotInRoom,

    /// Client is already in a room
    #[error("Already in room")]
    AlreadyInRoom,
}

/// Message send errors
///
/// Occurs when attempting to send messages through closed channels.
#[derive(Debug, Error)]
pub enum SendError {
    /// The receiving end of the channel has been closed
    #[error("Channel closed")]
    ChannelClosed,
}
