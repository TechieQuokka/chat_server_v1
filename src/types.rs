//! Basic type definitions for the chat server
//!
//! Provides newtype wrappers for type safety:
//! - `ClientId`: UUID-based unique client identifier
//! - `RoomCode`: 6-character alphanumeric room code

use uuid::Uuid;

/// Unique client identifier (newtype pattern)
///
/// Wraps a UUID v4 for type-safe client identification.
/// Implements Hash and Eq for use as HashMap keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub Uuid);

impl ClientId {
    /// Create a new random client ID
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

/// Room code (6-character uppercase alphanumeric)
///
/// Used to identify and join chat rooms.
/// Generated randomly or parsed from user input.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoomCode(pub String);

impl RoomCode {
    /// Generate a new random 6-character room code
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

    /// Create a RoomCode from a string (converts to uppercase)
    pub fn from_string(code: String) -> Self {
        Self(code.to_uppercase())
    }
}

impl std::fmt::Display for RoomCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_id_unique() {
        let id1 = ClientId::new();
        let id2 = ClientId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_room_code_length() {
        let code = RoomCode::generate();
        assert_eq!(code.0.len(), 6);
    }

    #[test]
    fn test_room_code_uppercase() {
        let code = RoomCode::from_string("abc123".to_string());
        assert_eq!(code.0, "ABC123");
    }
}
