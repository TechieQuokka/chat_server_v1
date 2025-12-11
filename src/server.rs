//! ChatServer Actor implementation
//!
//! The central actor that manages all state: clients, rooms, and client-room mappings.
//! Uses the Actor pattern with mpsc channels for message passing.

use std::collections::HashMap;

use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::client::Client;
use crate::error::AppError;
use crate::message::ServerMessage;
use crate::room::Room;
use crate::types::{ClientId, RoomCode};

/// Commands sent from handlers to the ChatServer actor
#[derive(Debug)]
pub enum ServerCommand {
    /// New client connected
    Connect {
        client_id: ClientId,
        sender: mpsc::Sender<ServerMessage>,
    },
    /// Client disconnected
    Disconnect {
        client_id: ClientId,
    },
    /// Set client's username
    SetUsername {
        client_id: ClientId,
        username: String,
    },
    /// Create a new room
    CreateRoom {
        client_id: ClientId,
    },
    /// Join an existing room
    JoinRoom {
        client_id: ClientId,
        room_code: String,
    },
    /// Send a chat message
    Chat {
        client_id: ClientId,
        content: String,
    },
    /// Client started typing
    Typing {
        client_id: ClientId,
    },
    /// Client stopped typing
    StopTyping {
        client_id: ClientId,
    },
    /// Leave the current room
    LeaveRoom {
        client_id: ClientId,
    },
}

/// The main ChatServer actor
///
/// Manages all state and processes commands from client handlers.
/// Uses HashMap for O(1) lookups on clients, rooms, and client-room mappings.
pub struct ChatServer {
    /// All connected clients: ClientId -> Client
    clients: HashMap<ClientId, Client>,
    /// All active rooms: RoomCode -> Room
    rooms: HashMap<RoomCode, Room>,
    /// Client to room mapping for fast lookup: ClientId -> RoomCode
    client_rooms: HashMap<ClientId, RoomCode>,
    /// Command receiver channel
    receiver: mpsc::Receiver<ServerCommand>,
}

impl ChatServer {
    /// Create a new ChatServer with the given command receiver
    pub fn new(receiver: mpsc::Receiver<ServerCommand>) -> Self {
        Self {
            clients: HashMap::new(),
            rooms: HashMap::new(),
            client_rooms: HashMap::new(),
            receiver,
        }
    }

    /// Run the ChatServer event loop
    ///
    /// Continuously receives and processes commands until all senders are dropped.
    pub async fn run(mut self) {
        info!("ChatServer started");

        while let Some(cmd) = self.receiver.recv().await {
            self.handle_command(cmd).await;
        }

        info!("ChatServer shutting down");
    }

    /// Process a single command
    async fn handle_command(&mut self, cmd: ServerCommand) {
        match cmd {
            ServerCommand::Connect { client_id, sender } => {
                self.handle_connect(client_id, sender).await;
            }
            ServerCommand::Disconnect { client_id } => {
                self.handle_disconnect(client_id).await;
            }
            ServerCommand::SetUsername { client_id, username } => {
                self.handle_set_username(client_id, username).await;
            }
            ServerCommand::CreateRoom { client_id } => {
                self.handle_create_room(client_id).await;
            }
            ServerCommand::JoinRoom { client_id, room_code } => {
                self.handle_join_room(client_id, room_code).await;
            }
            ServerCommand::Chat { client_id, content } => {
                self.handle_chat(client_id, content).await;
            }
            ServerCommand::Typing { client_id } => {
                self.handle_typing(client_id).await;
            }
            ServerCommand::StopTyping { client_id } => {
                self.handle_stop_typing(client_id).await;
            }
            ServerCommand::LeaveRoom { client_id } => {
                self.handle_leave_room(client_id).await;
            }
        }
    }

    /// Handle new client connection
    async fn handle_connect(&mut self, client_id: ClientId, sender: mpsc::Sender<ServerMessage>) {
        info!("Client {} connected", client_id);
        let client = Client::new(client_id, sender);
        self.clients.insert(client_id, client);
        debug!(
            "Total clients: {}, Total rooms: {}",
            self.clients.len(),
            self.rooms.len()
        );
    }

    /// Handle client disconnection
    async fn handle_disconnect(&mut self, client_id: ClientId) {
        info!("Client {} disconnected", client_id);

        // Remove from room if in one
        if let Some(room_code) = self.client_rooms.remove(&client_id) {
            self.remove_client_from_room(client_id, &room_code).await;
        }

        // Remove client
        self.clients.remove(&client_id);

        debug!(
            "Total clients: {}, Total rooms: {}",
            self.clients.len(),
            self.rooms.len()
        );
    }

    /// Handle username setting
    async fn handle_set_username(&mut self, client_id: ClientId, username: String) {
        let Some(client) = self.clients.get_mut(&client_id) else {
            return;
        };

        client.set_username(username.clone());
        info!("Client {} set username to '{}'", client_id, username);

        let _ = client
            .send(ServerMessage::UsernameSet {
                username: username.clone(),
            })
            .await;
    }

    /// Handle room creation
    async fn handle_create_room(&mut self, client_id: ClientId) {
        let Some(client) = self.clients.get(&client_id) else {
            return;
        };

        // Check username
        if !client.has_username() {
            let _ = client.send(AppError::UsernameRequired.into()).await;
            return;
        }

        // Check if already in a room
        if self.client_rooms.contains_key(&client_id) {
            let _ = client.send(AppError::AlreadyInRoom.into()).await;
            return;
        }

        // Generate unique room code
        let room_code = loop {
            let code = RoomCode::generate();
            if !self.rooms.contains_key(&code) {
                break code;
            }
        };

        // Create room
        let room = Room::new(room_code.clone(), client_id);
        self.rooms.insert(room_code.clone(), room);
        self.client_rooms.insert(client_id, room_code.clone());

        info!("Client {} created room {}", client_id, room_code);

        let _ = client
            .send(ServerMessage::RoomCreated {
                room_code: room_code.to_string(),
            })
            .await;
    }

    /// Handle room joining
    async fn handle_join_room(&mut self, client_id: ClientId, room_code: String) {
        let Some(client) = self.clients.get(&client_id) else {
            return;
        };

        // Check username
        if !client.has_username() {
            let _ = client.send(AppError::UsernameRequired.into()).await;
            return;
        }

        // Check if already in a room
        if self.client_rooms.contains_key(&client_id) {
            let _ = client.send(AppError::AlreadyInRoom.into()).await;
            return;
        }

        let room_code = RoomCode::from_string(room_code);

        // Check room exists
        let Some(room) = self.rooms.get_mut(&room_code) else {
            let _ = client
                .send(AppError::RoomNotFound(room_code.to_string()).into())
                .await;
            return;
        };

        // Check room capacity
        if room.is_full() {
            let _ = client.send(AppError::RoomFull.into()).await;
            return;
        }

        // Add guest to room
        let host_id = room.host;
        room.add_guest(client_id);
        self.client_rooms.insert(client_id, room_code.clone());

        info!("Client {} joined room {}", client_id, room_code);

        // Get host name
        let host_name = self
            .clients
            .get(&host_id)
            .and_then(|c| c.username.clone());

        // Notify joiner
        let _ = client
            .send(ServerMessage::RoomJoined {
                room_code: room_code.to_string(),
                partner: host_name,
            })
            .await;

        // Notify host
        if let Some(host) = self.clients.get(&host_id) {
            let guest_name = client.username.clone().unwrap_or_default();
            let _ = host
                .send(ServerMessage::PartnerJoined {
                    username: guest_name,
                })
                .await;
        }
    }

    /// Handle chat message
    async fn handle_chat(&mut self, client_id: ClientId, content: String) {
        let Some(client) = self.clients.get_mut(&client_id) else {
            return;
        };

        // Check if in a room
        let Some(room_code) = self.client_rooms.get(&client_id) else {
            let _ = client.send(AppError::NotInRoom.into()).await;
            return;
        };

        let room_code = room_code.clone();

        // Get sender name and clear typing status
        let sender_name = client.display_name().to_string();
        let was_typing = client.is_typing;
        client.set_typing(false);

        // Get room and partner
        let Some(room) = self.rooms.get(&room_code) else {
            return;
        };

        let Some(partner_id) = room.get_partner(client_id) else {
            return; // No partner to send to
        };

        // Send to partner
        if let Some(partner) = self.clients.get(&partner_id) {
            // Send stop typing if was typing
            if was_typing {
                let _ = partner.send(ServerMessage::PartnerStopTyping).await;
            }

            let _ = partner
                .send(ServerMessage::Chat {
                    from: sender_name,
                    content,
                })
                .await;
        }
    }

    /// Handle typing indicator start
    async fn handle_typing(&mut self, client_id: ClientId) {
        let Some(client) = self.clients.get_mut(&client_id) else {
            return;
        };

        // Check if in a room
        let Some(room_code) = self.client_rooms.get(&client_id) else {
            let _ = client.send(AppError::NotInRoom.into()).await;
            return;
        };

        let room_code = room_code.clone();

        // Already typing? Skip
        if client.is_typing {
            return;
        }

        client.set_typing(true);

        // Notify partner
        if let Some(partner_id) = self.get_partner(client_id, &room_code) {
            if let Some(partner) = self.clients.get(&partner_id) {
                let _ = partner.send(ServerMessage::PartnerTyping).await;
            }
        }
    }

    /// Handle typing indicator stop
    async fn handle_stop_typing(&mut self, client_id: ClientId) {
        let Some(client) = self.clients.get_mut(&client_id) else {
            return;
        };

        // Check if in a room
        let Some(room_code) = self.client_rooms.get(&client_id) else {
            return;
        };

        let room_code = room_code.clone();

        // Not typing? Skip
        if !client.is_typing {
            return;
        }

        client.set_typing(false);

        // Notify partner
        if let Some(partner_id) = self.get_partner(client_id, &room_code) {
            if let Some(partner) = self.clients.get(&partner_id) {
                let _ = partner.send(ServerMessage::PartnerStopTyping).await;
            }
        }
    }

    /// Handle voluntary room leaving
    async fn handle_leave_room(&mut self, client_id: ClientId) {
        let Some(client) = self.clients.get(&client_id) else {
            return;
        };

        // Check if in a room
        let Some(room_code) = self.client_rooms.remove(&client_id) else {
            let _ = client.send(AppError::NotInRoom.into()).await;
            return;
        };

        info!("Client {} left room {}", client_id, room_code);

        self.remove_client_from_room(client_id, &room_code).await;
    }

    /// Helper: Remove a client from their room and handle cleanup
    async fn remove_client_from_room(&mut self, client_id: ClientId, room_code: &RoomCode) {
        let Some(room) = self.rooms.get_mut(room_code) else {
            return;
        };

        // Get partner before removing
        let partner_id = room.get_partner(client_id);

        // Remove client from room
        let should_delete = room.remove_client(client_id);

        if should_delete {
            self.rooms.remove(room_code);
            debug!("Room {} deleted (empty)", room_code);
        }

        // Notify partner
        if let Some(partner_id) = partner_id {
            if let Some(partner) = self.clients.get(&partner_id) {
                let _ = partner.send(ServerMessage::PartnerLeft).await;
            }
        }
    }

    /// Helper: Get partner ID for a client in a room
    fn get_partner(&self, client_id: ClientId, room_code: &RoomCode) -> Option<ClientId> {
        self.rooms.get(room_code).and_then(|r| r.get_partner(client_id))
    }
}
