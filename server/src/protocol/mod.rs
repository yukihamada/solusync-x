use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod messages;

pub use messages::*;

/// Node types in the SOLUSync-X cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Master,
    Replica,
    Client,
}

/// Network quality indicators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkQuality {
    Excellent, // < 10ms RTT, 0% loss
    Good,      // < 50ms RTT, < 0.1% loss
    Fair,      // < 100ms RTT, < 1% loss
    Poor,      // < 200ms RTT, < 5% loss
    Critical,  // > 200ms RTT or > 5% loss
}

impl NetworkQuality {
    /// Calculate network quality from RTT and packet loss
    pub fn from_metrics(rtt_ms: f64, loss_percent: f64) -> Self {
        match (rtt_ms, loss_percent) {
            (r, l) if r < 10.0 && l == 0.0 => Self::Excellent,
            (r, l) if r < 50.0 && l < 0.1 => Self::Good,
            (r, l) if r < 100.0 && l < 1.0 => Self::Fair,
            (r, l) if r < 200.0 && l < 5.0 => Self::Poor,
            _ => Self::Critical,
        }
    }

    /// Get recommended future buffer size for this quality
    pub fn recommended_buffer_ms(&self) -> u64 {
        match self {
            Self::Excellent => 30,
            Self::Good => 80,
            Self::Fair => 120,
            Self::Poor => 180,
            Self::Critical => 250,
        }
    }
}

/// Common header for all SOLUSync-X messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    pub id: Uuid,
    pub timestamp: f64,
    pub node_id: Uuid,
    pub sequence: u64,
}

impl MessageHeader {
    pub fn new(node_id: Uuid, sequence: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: get_current_time(),
            node_id,
            sequence,
        }
    }
}

/// Get current time in seconds with microsecond precision
pub fn get_current_time() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs_f64()
}