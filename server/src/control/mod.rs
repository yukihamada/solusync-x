use anyhow::Result;
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::sync::RwLock;
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

pub mod handlers;

use crate::{
    clock::ClockManager,
    media::MediaServer,
    protocol::{
        ErrorCode, ErrorMessage, HelloMessage, Message as ProtoMessage, MessageHeader, NodeType,
    },
};

/// Control server for handling WebSocket connections and commands
pub struct ControlServer {
    /// Server ID
    server_id: Uuid,
    
    /// Clock manager
    clock_manager: Arc<ClockManager>,
    
    /// Media server
    media_server: Arc<MediaServer>,
    
    /// Connected clients
    clients: Arc<RwLock<HashMap<Uuid, ClientConnection>>>,
}

/// Connected client information
struct ClientConnection {
    client_id: Uuid,
    node_type: NodeType,
    tx: mpsc::Sender<ProtoMessage>,
    capabilities: Vec<String>,
}

impl ControlServer {
    pub fn new(clock_manager: Arc<ClockManager>, media_server: Arc<MediaServer>) -> Self {
        Self {
            server_id: Uuid::new_v4(),
            clock_manager,
            media_server,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Handle new WebSocket connection
    pub async fn handle_connection(&self, websocket: WebSocket) -> Result<()> {
        let (mut ws_sender, mut ws_receiver) = websocket.split();
        let (tx, mut rx) = mpsc::channel::<ProtoMessage>(100);
        
        let client_id = Uuid::new_v4();
        info!("New WebSocket connection: {}", client_id);
        
        // Spawn task to forward messages to WebSocket
        let tx_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let json = match serde_json::to_string(&msg) {
                    Ok(json) => json,
                    Err(e) => {
                        error!("Failed to serialize message: {}", e);
                        continue;
                    }
                };
                
                if ws_sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        });
        
        // Handle incoming messages
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    if let Err(e) = self.handle_message(&client_id, &text, &tx).await {
                        error!("Error handling message from {}: {}", client_id, e);
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Client {} disconnected", client_id);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error for {}: {}", client_id, e);
                    break;
                }
                _ => {}
            }
        }
        
        // Cleanup
        self.remove_client(&client_id).await;
        tx_task.abort();
        
        Ok(())
    }
    
    /// Handle incoming message
    async fn handle_message(
        &self,
        client_id: &Uuid,
        text: &str,
        tx: &mpsc::Sender<ProtoMessage>,
    ) -> Result<()> {
        let message: ProtoMessage = serde_json::from_str(text)?;
        
        match message {
            ProtoMessage::Hello(hello) => {
                self.handle_hello(client_id, hello, tx.clone()).await?;
            }
            ProtoMessage::ClockSync(sync) => {
                self.handle_clock_sync(client_id, sync, tx).await?;
            }
            ProtoMessage::MediaControl(control) => {
                self.handle_media_control(control).await?;
            }
            ProtoMessage::Heartbeat(heartbeat) => {
                self.handle_heartbeat(heartbeat, tx).await?;
            }
            _ => {
                warn!("Unhandled message type from {}", client_id);
            }
        }
        
        Ok(())
    }
    
    /// Handle hello message
    async fn handle_hello(
        &self,
        client_id: &Uuid,
        hello: HelloMessage,
        tx: mpsc::Sender<ProtoMessage>,
    ) -> Result<()> {
        info!(
            "Client {} hello: type={:?}, capabilities={:?}",
            client_id, hello.node_type, hello.capabilities
        );
        
        // TODO: Authenticate client
        
        // Store client connection
        let client = ClientConnection {
            client_id: *client_id,
            node_type: hello.node_type,
            tx: tx.clone(),
            capabilities: hello.capabilities,
        };
        
        self.clients.write().await.insert(*client_id, client);
        
        // Add to media server if client supports media
        self.media_server.add_client(*client_id).await?;
        
        // Send welcome response
        let response = ProtoMessage::Hello(HelloMessage {
            header: MessageHeader::new(self.server_id, 0),
            protocol_version: "0.1.0".to_string(),
            capabilities: vec![
                "clock_sync".to_string(),
                "media_streaming".to_string(),
                "cluster".to_string(),
            ],
            node_type: NodeType::Master,
            auth_token: None,
        });
        
        tx.send(response).await?;
        
        Ok(())
    }
    
    /// Handle clock sync
    async fn handle_clock_sync(
        &self,
        client_id: &Uuid,
        sync: crate::protocol::ClockSyncMessage,
        tx: &mpsc::Sender<ProtoMessage>,
    ) -> Result<()> {
        let response = crate::clock::ClockSync::create_response(&sync);
        tx.send(ProtoMessage::ClockSyncResponse(response)).await?;
        Ok(())
    }
    
    /// Handle media control
    async fn handle_media_control(
        &self,
        control: crate::protocol::MediaControlMessage,
    ) -> Result<()> {
        self.media_server
            .get_control_sender()
            .send(control)
            .await?;
        Ok(())
    }
    
    /// Handle heartbeat
    async fn handle_heartbeat(
        &self,
        heartbeat: crate::protocol::HeartbeatMessage,
        tx: &mpsc::Sender<ProtoMessage>,
    ) -> Result<()> {
        let mut response = heartbeat.clone();
        response.server_time = Some(self.clock_manager.now().await);
        tx.send(ProtoMessage::Heartbeat(response)).await?;
        Ok(())
    }
    
    /// Remove client
    async fn remove_client(&self, client_id: &Uuid) {
        self.clients.write().await.remove(client_id);
        info!("Removed client: {}", client_id);
    }
    
    /// Send error to client
    async fn send_error(
        &self,
        client_id: &Uuid,
        code: ErrorCode,
        message: String,
    ) -> Result<()> {
        if let Some(client) = self.clients.read().await.get(client_id) {
            let error = ProtoMessage::Error(ErrorMessage {
                header: MessageHeader::new(self.server_id, 0),
                code,
                message,
                details: None,
            });
            
            client.tx.send(error).await?;
        }
        
        Ok(())
    }
    
    /// Broadcast message to all clients
    pub async fn broadcast(&self, message: ProtoMessage) -> Result<()> {
        let clients = self.clients.read().await;
        
        for (client_id, client) in clients.iter() {
            if let Err(e) = client.tx.send(message.clone()).await {
                warn!("Failed to send to client {}: {}", client_id, e);
            }
        }
        
        Ok(())
    }
}