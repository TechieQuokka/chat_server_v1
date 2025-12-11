//! Room struct definition
//!
//! Represents a 1:1 chat room with host and optional guest.

use std::time::Instant;

use crate::types::{ClientId, RoomCode};

/// 1:1 Chat Room
///
/// A room can have at most 2 participants: a host (creator) and a guest.
/// The host is promoted when the original host leaves.
#[derive(Debug)]
pub struct Room {
    /// Room code for identification
    pub code: RoomCode,
    /// Room creator (host)
    pub host: ClientId,
    /// Joined partner (guest)
    pub guest: Option<ClientId>,
    /// Room creation time
    pub created_at: Instant,
}

impl Room {
    /// Create a new room with the given code and host
    pub fn new(code: RoomCode, host: ClientId) -> Self {
        Self {
            code,
            host,
            guest: None,
            created_at: Instant::now(),
        }
    }

    /// Check if room is full (2 people)
    pub fn is_full(&self) -> bool {
        self.guest.is_some()
    }

    /// Check if room is empty (only host, no guest)
    pub fn is_empty(&self) -> bool {
        self.guest.is_none()
    }

    /// Get the partner's ClientId for a given client
    ///
    /// Returns None if the client is not in the room or has no partner.
    pub fn get_partner(&self, client_id: ClientId) -> Option<ClientId> {
        if self.host == client_id {
            self.guest
        } else if self.guest == Some(client_id) {
            Some(self.host)
        } else {
            None
        }
    }

    /// Check if a client is in this room
    pub fn contains(&self, client_id: ClientId) -> bool {
        self.host == client_id || self.guest == Some(client_id)
    }

    /// Remove a client from the room (handle leaving)
    ///
    /// Returns true if the room should be deleted (no participants left).
    /// If the host leaves, the guest is promoted to host.
    pub fn remove_client(&mut self, client_id: ClientId) -> bool {
        if self.host == client_id {
            // If host leaves, promote guest to host
            if let Some(guest) = self.guest.take() {
                self.host = guest;
                false // Keep room
            } else {
                true // Delete room (no one left)
            }
        } else if self.guest == Some(client_id) {
            self.guest = None;
            false // Keep room (host remains)
        } else {
            false // Client wasn't in room
        }
    }

    /// Add a guest to the room
    ///
    /// Returns false if the room is already full.
    pub fn add_guest(&mut self, guest_id: ClientId) -> bool {
        if self.is_full() {
            false
        } else {
            self.guest = Some(guest_id);
            true
        }
    }

    /// Get the number of participants in the room
    pub fn participant_count(&self) -> usize {
        if self.guest.is_some() {
            2
        } else {
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_creation() {
        let host_id = ClientId::new();
        let code = RoomCode::generate();
        let room = Room::new(code.clone(), host_id);

        assert_eq!(room.code, code);
        assert_eq!(room.host, host_id);
        assert!(room.guest.is_none());
        assert!(!room.is_full());
        assert!(room.is_empty());
        assert_eq!(room.participant_count(), 1);
    }

    #[test]
    fn test_room_guest_join() {
        let host_id = ClientId::new();
        let guest_id = ClientId::new();
        let mut room = Room::new(RoomCode::generate(), host_id);

        assert!(room.add_guest(guest_id));
        assert!(room.is_full());
        assert!(!room.is_empty());
        assert_eq!(room.participant_count(), 2);

        // Cannot add another guest
        let another_id = ClientId::new();
        assert!(!room.add_guest(another_id));
    }

    #[test]
    fn test_room_get_partner() {
        let host_id = ClientId::new();
        let guest_id = ClientId::new();
        let mut room = Room::new(RoomCode::generate(), host_id);

        // No partner before guest joins
        assert!(room.get_partner(host_id).is_none());

        room.add_guest(guest_id);

        // Both can find their partner
        assert_eq!(room.get_partner(host_id), Some(guest_id));
        assert_eq!(room.get_partner(guest_id), Some(host_id));

        // Unknown client has no partner
        let unknown_id = ClientId::new();
        assert!(room.get_partner(unknown_id).is_none());
    }

    #[test]
    fn test_room_contains() {
        let host_id = ClientId::new();
        let guest_id = ClientId::new();
        let other_id = ClientId::new();
        let mut room = Room::new(RoomCode::generate(), host_id);

        assert!(room.contains(host_id));
        assert!(!room.contains(guest_id));
        assert!(!room.contains(other_id));

        room.add_guest(guest_id);

        assert!(room.contains(host_id));
        assert!(room.contains(guest_id));
        assert!(!room.contains(other_id));
    }

    #[test]
    fn test_room_guest_leaves() {
        let host_id = ClientId::new();
        let guest_id = ClientId::new();
        let mut room = Room::new(RoomCode::generate(), host_id);
        room.add_guest(guest_id);

        // Guest leaves
        let should_delete = room.remove_client(guest_id);
        assert!(!should_delete);
        assert!(room.guest.is_none());
        assert_eq!(room.host, host_id);
    }

    #[test]
    fn test_room_host_leaves_with_guest() {
        let host_id = ClientId::new();
        let guest_id = ClientId::new();
        let mut room = Room::new(RoomCode::generate(), host_id);
        room.add_guest(guest_id);

        // Host leaves - guest promoted to host
        let should_delete = room.remove_client(host_id);
        assert!(!should_delete);
        assert_eq!(room.host, guest_id);
        assert!(room.guest.is_none());
    }

    #[test]
    fn test_room_host_leaves_alone() {
        let host_id = ClientId::new();
        let mut room = Room::new(RoomCode::generate(), host_id);

        // Host leaves alone - room should be deleted
        let should_delete = room.remove_client(host_id);
        assert!(should_delete);
    }
}
