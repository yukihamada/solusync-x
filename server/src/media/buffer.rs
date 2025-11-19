use std::time::{Duration, Instant};
use crate::protocol::NetworkQuality;

/// Media frame with timing information
#[derive(Debug, Clone)]
pub struct MediaFrame {
    /// Frame data
    pub data: Vec<u8>,
    
    /// Presentation timestamp (network clock)
    pub timestamp: f64,
    
    /// Frame duration
    pub duration: Duration,
    
    /// Frame type
    pub frame_type: FrameType,
    
    /// Sequence number
    pub sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Audio,
    Video,
    VideoKeyframe,
}

/// Dynamic future buffer that adjusts based on network conditions
pub struct DynamicFutureBuffer {
    /// Target latency for future playback
    target_latency: Duration,
    
    /// Minimum latency (best case)
    min_latency: Duration,
    
    /// Maximum latency (worst case)
    max_latency: Duration,
    
    /// Current network quality
    network_quality: NetworkQuality,
    
    /// Latency adjustment rate
    adjustment_rate: f64,
    
    /// Last adjustment time
    last_adjustment: Instant,
    
    /// Statistics
    underrun_count: u64,
    overrun_count: u64,
}

impl DynamicFutureBuffer {
    pub fn new(initial_latency: Duration, quality: NetworkQuality) -> Self {
        Self {
            target_latency: initial_latency,
            min_latency: Duration::from_millis(30),
            max_latency: Duration::from_millis(500),
            network_quality: quality,
            adjustment_rate: 0.1, // 10% adjustment per update
            last_adjustment: Instant::now(),
            underrun_count: 0,
            overrun_count: 0,
        }
    }
    
    /// Update network quality and adjust buffer
    pub fn update_network_quality(&mut self, quality: NetworkQuality) {
        self.network_quality = quality;
        
        // Only adjust if enough time has passed
        if self.last_adjustment.elapsed() < Duration::from_millis(500) {
            return;
        }
        
        let recommended = Duration::from_millis(quality.recommended_buffer_ms());
        self.adjust_target_latency(recommended);
        self.last_adjustment = Instant::now();
    }
    
    /// Get current target latency
    pub fn target_latency(&self) -> f64 {
        self.target_latency.as_secs_f64()
    }
    
    /// Report buffer underrun (playback starvation)
    pub fn report_underrun(&mut self) {
        self.underrun_count += 1;
        
        // Increase buffer size
        let new_target = self.target_latency.mul_f64(1.0 + self.adjustment_rate);
        self.target_latency = new_target.min(self.max_latency);
        
        tracing::warn!(
            "Buffer underrun! Increasing latency to {}ms",
            self.target_latency.as_millis()
        );
    }
    
    /// Report buffer overrun (too much latency)
    pub fn report_overrun(&mut self) {
        self.overrun_count += 1;
        
        // Decrease buffer size slowly
        let new_target = self.target_latency.mul_f64(1.0 - self.adjustment_rate * 0.5);
        self.target_latency = new_target.max(self.min_latency);
        
        tracing::debug!(
            "Buffer overrun. Decreasing latency to {}ms",
            self.target_latency.as_millis()
        );
    }
    
    /// Calculate jitter buffer depth based on statistics
    pub fn calculate_jitter_buffer(&self) -> Duration {
        match self.network_quality {
            NetworkQuality::Excellent => Duration::from_millis(5),
            NetworkQuality::Good => Duration::from_millis(10),
            NetworkQuality::Fair => Duration::from_millis(20),
            NetworkQuality::Poor => Duration::from_millis(40),
            NetworkQuality::Critical => Duration::from_millis(80),
        }
    }
    
    /// Get buffer statistics
    pub fn stats(&self) -> BufferStats {
        BufferStats {
            target_latency_ms: self.target_latency.as_millis() as u32,
            underrun_count: self.underrun_count,
            overrun_count: self.overrun_count,
            network_quality: self.network_quality,
        }
    }
    
    /// Adjust target latency towards recommended value
    fn adjust_target_latency(&mut self, recommended: Duration) {
        let current = self.target_latency.as_secs_f64();
        let target = recommended.as_secs_f64();
        
        // Smooth adjustment using exponential moving average
        let new_latency = current * (1.0 - self.adjustment_rate) + target * self.adjustment_rate;
        
        self.target_latency = Duration::from_secs_f64(new_latency)
            .clamp(self.min_latency, self.max_latency);
        
        tracing::debug!(
            "Adjusted buffer latency: {}ms -> {}ms (recommended: {}ms)",
            (current * 1000.0) as u32,
            self.target_latency.as_millis(),
            recommended.as_millis()
        );
    }
}

#[derive(Debug, Clone)]
pub struct BufferStats {
    pub target_latency_ms: u32,
    pub underrun_count: u64,
    pub overrun_count: u64,
    pub network_quality: NetworkQuality,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dynamic_buffer_adjustment() {
        let mut buffer = DynamicFutureBuffer::new(
            Duration::from_millis(80),
            NetworkQuality::Good,
        );
        
        // Simulate underruns
        buffer.report_underrun();
        assert!(buffer.target_latency > Duration::from_millis(80));
        
        // Simulate network quality change
        buffer.update_network_quality(NetworkQuality::Poor);
        std::thread::sleep(Duration::from_millis(600)); // Wait for adjustment
        buffer.update_network_quality(NetworkQuality::Poor);
        
        // Should adjust towards poor network recommendation
        assert!(buffer.target_latency > Duration::from_millis(150));
    }
}