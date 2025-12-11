//! Client struct definition
//!
//! Represents a connected client with their state and communication channel.

use tokio::sync::mpsc;

use crate::error::SendError;
use crate::message::ServerMessage;
use crate::types::ClientId;

/// Connected client information
///
/// Holds all state related to a connected client including their
/// unique ID, username, message sender channel, and typing status.
#[derive(Debug)]
pub struct Client {
    /// Unique identifier for this client
    pub id: ClientId,
    /// Username (None before setup)
    pub username: Option<String>,
    /// Server → Client message channel
    pub sender: mpsc::Sender<ServerMessage>,
    /// Currently typing flag
    pub is_typing: bool,
}

impl Client {
    /// Create a new client with the given ID and sender channel
    pub fn new(id: ClientId, sender: mpsc::Sender<ServerMessage>) -> Self {
        Self {
            id,
            username: None,
            sender,
            is_typing: false,
        }
    }

    /// Send a message to this client
    ///
    /// Returns an error if the channel is closed (client disconnected).
    pub async fn send(&self, msg: ServerMessage) -> Result<(), SendError> {
        self.sender
            .send(msg)
            .await
            .map_err(|_| SendError::ChannelClosed)
    }

    /// Get the display name for this client
    ///
    /// Returns the username if set, otherwise "Unknown".
    pub fn display_name(&self) -> &str {
        self.username.as_deref().unwrap_or("Unknown")
    }

    /// Check if this client has set their username
    pub fn has_username(&self) -> bool {
        self.username.is_some()
    }

    /// Set the client's username
    pub fn set_username(&mut self, username: String) {
        self.username = Some(username);
    }

    /// Set typing status
    pub fn set_typing(&mut self, is_typing: bool) {
        self.is_typing = is_typing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let (tx, _rx) = mpsc::channel(32);
        let client = Client::new(ClientId::new(), tx);

        assert!(client.username.is_none());
        assert!(!client.is_typing);
        assert_eq!(client.display_name(), "Unknown");
    }

    #[tokio::test]
    async fn test_client_username() {
        let (tx, _rx) = mpsc::channel(32);
        let mut client = Client::new(ClientId::new(), tx);

        assert!(!client.has_username());

        client.set_username("Alice".to_string());

        assert!(client.has_username());
        assert_eq!(client.display_name(), "Alice");
    }
}
