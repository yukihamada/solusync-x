use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{MessageHeader, NetworkQuality, NodeType};

/// All possible messages in the SOLUSync-X protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    // Clock synchronization
    ClockSync(ClockSyncMessage),
    ClockSyncResponse(ClockSyncResponse),
    
    // Media control
    MediaControl(MediaControlMessage),
    MediaData(MediaDataMessage),
    
    // Cluster management
    NodeAnnounce(NodeAnnounceMessage),
    NodeStatus(NodeStatusMessage),
    MasterElection(MasterElectionMessage),
    
    // Connection
    Hello(HelloMessage),
    Heartbeat(HeartbeatMessage),
    Error(ErrorMessage),
}

/// Initial handshake message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloMessage {
    pub header: MessageHeader,
    pub protocol_version: String,
    pub capabilities: Vec<String>,
    pub node_type: NodeType,
    pub auth_token: Option<String>,
}

/// Clock synchronization request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockSyncMessage {
    pub header: MessageHeader,
    pub t1: f64, // Client timestamp when sending
}

/// Clock synchronization response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockSyncResponse {
    pub header: MessageHeader,
    pub t1: f64, // Original client timestamp
    pub t2: f64, // Server timestamp when received
    pub t3: f64, // Server timestamp when sending response
}

/// Media control commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaControlMessage {
    pub header: MessageHeader,
    pub action: MediaAction,
    pub track_id: String,
    pub start_at: f64, // Network clock time to start
    pub params: MediaParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaAction {
    Play,
    Pause,
    Stop,
    Seek,
    Load,
    Unload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaParams {
    pub volume: Option<f32>,
    pub loop_count: Option<u32>,
    pub fade_in_ms: Option<u32>,
    pub fade_out_ms: Option<u32>,
    pub seek_position: Option<f64>,
}

/// Media data chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaDataMessage {
    pub header: MessageHeader,
    pub track_id: String,
    pub chunk_index: u64,
    pub timestamp: f64,    // Presentation timestamp
    pub duration: f64,     // Duration of this chunk
    pub data: Vec<u8>,     // Encoded media data
    pub codec: String,     // e.g., "opus", "pcm16", "h264"
    pub is_keyframe: bool,
}

/// Node announcement for cluster discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAnnounceMessage {
    pub header: MessageHeader,
    pub node_type: NodeType,
    pub capabilities: Vec<String>,
    pub endpoint: String, // IP:Port or domain
    pub public_key: Option<Vec<u8>>,
}

/// Periodic node status update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatusMessage {
    pub header: MessageHeader,
    pub node_type: NodeType,
    pub connected_clients: u32,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub battery_level: Option<f32>,
    pub network_quality: NetworkQuality,
    pub avg_rtt_ms: f64,
    pub packet_loss_percent: f64,
    pub uptime_seconds: u64,
}

/// Master election message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterElectionMessage {
    pub header: MessageHeader,
    pub election_id: Uuid,
    pub candidate_score: f64,
    pub current_master: Option<Uuid>,
}

/// Heartbeat to keep connection alive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    pub header: MessageHeader,
    pub client_time: f64,
    pub server_time: Option<f64>,
}

/// Error message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub header: MessageHeader,
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    AuthenticationFailed = 401,
    Unauthorized = 403,
    NotFound = 404,
    RateLimited = 429,
    InternalError = 500,
    ProtocolError = 501,
    NetworkError = 502,
    ClockSyncFailed = 510,
    MediaError = 520,
    ClusterError = 530,
}