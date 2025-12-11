//! WebSocket connection handler
//!
//! Handles individual client connections: WebSocket handshake,
//! message parsing, and bidirectional communication with the ChatServer.

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

use crate::error::AppError;
use crate::message::{ClientMessage, ServerMessage};
use crate::server::ServerCommand;
use crate::types::ClientId;

/// Handle a new TCP connection
///
/// Performs WebSocket handshake, sets up bidirectional communication,
/// and manages the connection lifecycle.
pub async fn handle_connection(
    stream: TcpStream,
    cmd_tx: mpsc::Sender<ServerCommand>,
) -> Result<(), AppError> {
    let peer_addr = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    debug!("New TCP connection from {}", peer_addr);

    // WebSocket handshake
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Generate client ID
    let client_id = ClientId::new();
    info!("Client {} connected from {}", client_id, peer_addr);

    // Create channel for server -> client messages
    let (msg_tx, mut msg_rx) = mpsc::channel::<ServerMessage>(32);

    // Register with ChatServer
    if cmd_tx
        .send(ServerCommand::Connect {
            client_id,
            sender: msg_tx,
        })
        .await
        .is_err()
    {
        error!("Failed to register client {} - server closed", client_id);
        return Err(AppError::ChannelSend);
    }

    // Send connection success message
    let connected_msg = ServerMessage::Connected {
        client_id: client_id.to_string(),
    };
    let json = serde_json::to_string(&connected_msg)?;
    ws_sender.send(Message::Text(json.into())).await?;

    // Clone cmd_tx for read task
    let cmd_tx_read = cmd_tx.clone();

    // Spawn read task (WebSocket -> ServerCommand)
    let read_task = tokio::spawn(async move {
        while let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(client_msg) => {
                            let cmd = client_message_to_command(client_id, client_msg);
                            if cmd_tx_read.send(cmd).await.is_err() {
                                debug!("Server closed, ending read task for {}", client_id);
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("Invalid JSON from {}: {}", client_id, e);
                            // Note: We can't easily send an error back here
                            // as we don't have access to msg_tx in this task.
                            // The server should handle invalid messages gracefully.
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    debug!("Client {} sent close frame", client_id);
                    break;
                }
                Ok(Message::Ping(data)) => {
                    debug!("Ping from {}", client_id);
                    // Pong is handled automatically by tungstenite
                    let _ = data; // Suppress unused warning
                }
                Ok(Message::Pong(_)) => {
                    debug!("Pong from {}", client_id);
                }
                Ok(_) => {
                    // Binary or other message types - ignore
                }
                Err(e) => {
                    error!("WebSocket error for {}: {}", client_id, e);
                    break;
                }
            }
        }
        debug!("Read task ended for {}", client_id);
    });

    // Spawn write task (ServerMessage -> WebSocket)
    let write_task = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if ws_sender.send(Message::Text(json.into())).await.is_err() {
                        debug!("WebSocket send failed, ending write task");
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                    // Continue - don't break on serialization errors
                }
            }
        }
        debug!("Write task ended for client");

        // Send close frame when done
        let _ = ws_sender.close().await;
    });

    // Wait for either task to complete
    tokio::select! {
        _ = read_task => {
            debug!("Read task completed for {}", client_id);
        }
        _ = write_task => {
            debug!("Write task completed for {}", client_id);
        }
    }

    // Send disconnect command
    let _ = cmd_tx
        .send(ServerCommand::Disconnect { client_id })
        .await;

    info!("Client {} disconnected", client_id);

    Ok(())
}

/// Convert a ClientMessage to a ServerCommand
fn client_message_to_command(client_id: ClientId, msg: ClientMessage) -> ServerCommand {
    match msg {
        ClientMessage::SetUsername { username } => ServerCommand::SetUsername { client_id, username },
        ClientMessage::CreateRoom => ServerCommand::CreateRoom { client_id },
        ClientMessage::JoinRoom { room_code } => ServerCommand::JoinRoom { client_id, room_code },
        ClientMessage::Chat { content } => ServerCommand::Chat { client_id, content },
        ClientMessage::Typing => ServerCommand::Typing { client_id },
        ClientMessage::StopTyping => ServerCommand::StopTyping { client_id },
        ClientMessage::LeaveRoom => ServerCommand::LeaveRoom { client_id },
    }
}
