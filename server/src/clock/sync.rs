use crate::protocol::{get_current_time, ClockSyncMessage, ClockSyncResponse};

/// Clock synchronization sample
#[derive(Debug, Clone, Copy)]
pub struct ClockSample {
    /// Calculated clock offset in seconds
    pub offset: f64,
    
    /// Round-trip time in seconds
    pub rtt: f64,
    
    /// Timestamp when sample was taken
    pub timestamp: f64,
}

/// Clock synchronization algorithm (PTP-inspired)
pub struct ClockSync;

impl ClockSync {
    /// Calculate clock offset from sync messages
    /// 
    /// Uses the standard NTP/PTP algorithm:
    /// - t1: Client send time
    /// - t2: Server receive time  
    /// - t3: Server send time
    /// - t4: Client receive time
    /// 
    /// offset = ((t2 - t1) + (t3 - t4)) / 2
    /// rtt = (t4 - t1) - (t3 - t2)
    pub fn calculate_offset(
        t1: f64,
        t2: f64,
        t3: f64,
        t4: f64,
    ) -> ClockSample {
        let rtt = (t4 - t1) - (t3 - t2);
        let offset = ((t2 - t1) + (t3 - t4)) / 2.0;
        
        ClockSample {
            offset,
            rtt,
            timestamp: t4,
        }
    }
    
    /// Create a sync response from a sync request
    pub fn create_response(msg: &ClockSyncMessage) -> ClockSyncResponse {
        let t2 = get_current_time();
        
        ClockSyncResponse {
            header: crate::protocol::MessageHeader::new(msg.header.node_id, 0),
            t1: msg.t1,
            t2,
            t3: get_current_time(), // Will be slightly after t2
        }
    }
    
    /// Process a sync response to get clock sample
    pub fn process_response(
        original_t1: f64,
        response: &ClockSyncResponse,
    ) -> ClockSample {
        let t4 = get_current_time();
        Self::calculate_offset(original_t1, response.t2, response.t3, t4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_offset_calculation() {
        // Simulate a scenario where server is 1 second ahead
        let t1 = 100.0; // Client sends
        let t2 = 101.5; // Server receives (1s ahead + 0.5s network)
        let t3 = 101.6; // Server sends (0.1s processing)
        let t4 = 101.1; // Client receives (0.5s network)
        
        let sample = ClockSync::calculate_offset(t1, t2, t3, t4);
        
        // Expected offset: ((101.5 - 100) + (101.6 - 101.1)) / 2 = (1.5 + 0.5) / 2 = 1.0
        assert!((sample.offset - 1.0).abs() < 0.001);
        
        // Expected RTT: (101.1 - 100) - (101.6 - 101.5) = 1.1 - 0.1 = 1.0
        assert!((sample.rtt - 1.0).abs() < 0.001);
    }
    
    #[test]
    fn test_symmetric_network_delay() {
        // When network delay is symmetric, offset calculation is accurate
        let t1 = 100.0;
        let t2 = 100.1; // 0.1s network delay
        let t3 = 100.2; // 0.1s processing
        let t4 = 100.3; // 0.1s network delay
        
        let sample = ClockSync::calculate_offset(t1, t2, t3, t4);
        
        // No clock offset when clocks are synchronized
        assert!(sample.offset.abs() < 0.001);
        assert!((sample.rtt - 0.2).abs() < 0.001);
    }
}