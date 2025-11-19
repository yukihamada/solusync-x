use anyhow::Result;
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info};
use uuid::Uuid;
use webrtc::peer_connection::RTCPeerConnection;

mod buffer;
mod webrtc_server;

pub use buffer::{DynamicFutureBuffer, MediaFrame};
pub use webrtc_server::WebRtcServer;

use crate::{
    clock::ClockManager,
    protocol::{MediaControlMessage, MediaDataMessage, NetworkQuality},
};

/// Manages media streaming and synchronization
pub struct MediaServer {
    /// Server ID
    server_id: Uuid,
    
    /// Clock manager reference
    clock_manager: Arc<ClockManager>,
    
    /// Active media streams
    streams: Arc<RwLock<HashMap<String, MediaStream>>>,
    
    /// Connected clients
    clients: Arc<RwLock<HashMap<Uuid, MediaClient>>>,
    
    /// WebRTC server
    webrtc_server: Arc<WebRtcServer>,
    
    /// Control command channel
    control_rx: Arc<RwLock<mpsc::Receiver<MediaControlMessage>>>,
    control_tx: mpsc::Sender<MediaControlMessage>,
}

/// Active media stream
struct MediaStream {
    track_id: String,
    codec: String,
    bitrate: u32,
    sample_rate: u32,
    channels: u8,
    /// Broadcast channel for media frames
    frame_tx: broadcast::Sender<MediaFrame>,
}

/// Connected media client
struct MediaClient {
    client_id: Uuid,
    peer_connection: Arc<RTCPeerConnection>,
    future_buffer: DynamicFutureBuffer,
    network_quality: NetworkQuality,
    subscribed_tracks: Vec<String>,
}

impl MediaServer {
    pub fn new() -> Self {
        let (control_tx, control_rx) = mpsc::channel(100);
        
        Self {
            server_id: Uuid::new_v4(),
            clock_manager: Arc::new(ClockManager::new()),
            streams: Arc::new(RwLock::new(HashMap::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            webrtc_server: Arc::new(WebRtcServer::new()),
            control_rx: Arc::new(RwLock::new(control_rx)),
            control_tx,
        }
    }
    
    /// Get command sender for external control
    pub fn get_control_sender(&self) -> mpsc::Sender<MediaControlMessage> {
        self.control_tx.clone()
    }
    
    /// Create a new media stream
    pub fn create_stream(&self, track_id: String, codec: String) -> Result<()> {
        let (frame_tx, _) = broadcast::channel(1000);
        
        let stream = MediaStream {
            track_id: track_id.clone(),
            codec,
            bitrate: 128000,
            sample_rate: 48000,
            channels: 2,
            frame_tx,
        };
        
        self.streams.write().insert(track_id.clone(), stream);
        info!("Created media stream: {}", track_id);
        
        Ok(())
    }
    
    /// Add media client
    pub async fn add_client(&self, client_id: Uuid) -> Result<()> {
        let peer_connection = self.webrtc_server.create_peer_connection().await?;
        
        let client = MediaClient {
            client_id,
            peer_connection,
            future_buffer: DynamicFutureBuffer::new(
                Duration::from_millis(80),
                NetworkQuality::Good,
            ),
            network_quality: NetworkQuality::Good,
            subscribed_tracks: Vec::new(),
        };
        
        self.clients.write().insert(client_id, client);
        info!("Added media client: {}", client_id);
        
        Ok(())
    }
    
    /// Update client network quality
    pub fn update_client_quality(&self, client_id: Uuid, quality: NetworkQuality) {
        if let Some(client) = self.clients.write().get_mut(&client_id) {
            client.network_quality = quality;
            client.future_buffer.update_network_quality(quality);
            
            debug!(
                "Updated client {} network quality: {:?}, buffer: {}ms",
                client_id,
                quality,
                quality.recommended_buffer_ms()
            );
        }
    }
    
    /// Subscribe client to a track
    pub fn subscribe_client(&self, client_id: Uuid, track_id: String) -> Result<()> {
        let streams = self.streams.read();
        let stream = streams
            .get(&track_id)
            .ok_or_else(|| anyhow::anyhow!("Track not found: {}", track_id))?;
        
        let mut frame_rx = stream.frame_tx.subscribe();
        
        // Spawn task to forward frames to client
        let clients = self.clients.clone();
        let clock = self.clock_manager.clone();
        
        tokio::spawn(async move {
            while let Ok(frame) = frame_rx.recv().await {
                if let Some(client) = clients.read().get(&client_id) {
                    // Add frame to future buffer with synchronized timestamp
                    let network_time = clock.now();
                    let future_time = network_time + client.future_buffer.target_latency();
                    
                    // TODO: Send frame via WebRTC
                    debug!(
                        "Scheduling frame for client {} at {:.3}",
                        client_id, future_time
                    );
                }
            }
        });
        
        if let Some(client) = self.clients.write().get_mut(&client_id) {
            client.subscribed_tracks.push(track_id);
        }
        
        Ok(())
    }
    
    /// Process media control command
    async fn process_control(&self, cmd: MediaControlMessage) -> Result<()> {
        use crate::protocol::MediaAction;
        
        match cmd.action {
            MediaAction::Play => {
                info!("Play track {} at {}", cmd.track_id, cmd.start_at);
                // TODO: Schedule playback
            }
            MediaAction::Pause => {
                info!("Pause track {}", cmd.track_id);
                // TODO: Pause playback
            }
            MediaAction::Stop => {
                info!("Stop track {}", cmd.track_id);
                // TODO: Stop playback
            }
            _ => {
                debug!("Unhandled media action: {:?}", cmd.action);
            }
        }
        
        Ok(())
    }
    
    /// Run the media server
    pub async fn run(self: Arc<Self>) {
        info!("Media server started");
        
        let mut stats_interval = tokio::time::interval(Duration::from_secs(5));
        
        loop {
            tokio::select! {
                _ = stats_interval.tick() => {
                    self.log_stats();
                }
                
                _ = self.process_commands() => {}
            }
        }
    }
    
    /// Process incoming commands
    async fn process_commands(&self) {
        let mut rx = self.control_rx.write();
        
        while let Some(cmd) = rx.recv().await {
            if let Err(e) = self.process_control(cmd).await {
                error!("Error processing control command: {}", e);
            }
        }
    }
    
    /// Log server statistics
    fn log_stats(&self) {
        let streams = self.streams.read();
        let clients = self.clients.read();
        
        debug!(
            "Media server stats: {} streams, {} clients",
            streams.len(),
            clients.len()
        );
    }
}