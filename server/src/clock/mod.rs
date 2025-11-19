use anyhow::Result;
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

mod filter;
mod sync;

pub use filter::KalmanFilter;
pub use sync::{ClockSample, ClockSync};

/// Manages clock synchronization for all connected nodes
pub struct ClockManager {
    /// Our node ID
    node_id: Uuid,
    
    /// Clock synchronization state for each peer
    peers: Arc<RwLock<HashMap<Uuid, PeerClock>>>,
    
    /// Master clock offset (if we're not the master)
    master_offset: Arc<RwLock<Option<f64>>>,
    
    /// Channel for clock sync samples
    sample_tx: mpsc::Sender<(Uuid, ClockSample)>,
    sample_rx: Arc<RwLock<mpsc::Receiver<(Uuid, ClockSample)>>>,
}

/// Clock state for a single peer
struct PeerClock {
    /// Kalman filter for smoothing clock offset
    filter: KalmanFilter,
    
    /// Last known offset in seconds
    offset: f64,
    
    /// Last RTT measurement
    rtt: f64,
    
    /// Last update time
    last_update: Instant,
    
    /// Number of samples received
    sample_count: u64,
    
    /// Clock drift rate (ppm)
    drift_ppm: f64,
}

impl ClockManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1000);
        
        Self {
            node_id: Uuid::new_v4(),
            peers: Arc::new(RwLock::new(HashMap::new())),
            master_offset: Arc::new(RwLock::new(None)),
            sample_tx: tx,
            sample_rx: Arc::new(RwLock::new(rx)),
        }
    }
    
    /// Get current synchronized time
    pub fn now(&self) -> f64 {
        let local_time = crate::protocol::get_current_time();
        
        // Apply master offset if we're not the master
        if let Some(offset) = *self.master_offset.read() {
            local_time + offset
        } else {
            local_time
        }
    }
    
    /// Submit a clock sample from a peer
    pub async fn add_sample(&self, peer_id: Uuid, sample: ClockSample) -> Result<()> {
        self.sample_tx.send((peer_id, sample)).await?;
        Ok(())
    }
    
    /// Get clock offset for a specific peer
    pub fn get_peer_offset(&self, peer_id: &Uuid) -> Option<f64> {
        self.peers.read().get(peer_id).map(|p| p.offset)
    }
    
    /// Get network statistics for a peer
    pub fn get_peer_stats(&self, peer_id: &Uuid) -> Option<(f64, f64, u64)> {
        self.peers.read().get(peer_id).map(|p| {
            (p.offset, p.rtt, p.sample_count)
        })
    }
    
    /// Run the clock manager background task
    pub async fn run(self: Arc<Self>) {
        info!("Clock manager started for node {}", self.node_id);
        
        let mut maintenance_interval = tokio::time::interval(Duration::from_secs(10));
        
        loop {
            tokio::select! {
                _ = maintenance_interval.tick() => {
                    self.cleanup_stale_peers();
                }
                
                _ = self.process_samples() => {}
            }
        }
    }
    
    /// Process incoming clock samples
    async fn process_samples(&self) {
        let mut rx = self.sample_rx.write();
        
        while let Some((peer_id, sample)) = rx.recv().await {
            self.update_peer_clock(peer_id, sample);
        }
    }
    
    /// Update clock state for a peer
    fn update_peer_clock(&self, peer_id: Uuid, sample: ClockSample) {
        let mut peers = self.peers.write();
        
        let peer = peers.entry(peer_id).or_insert_with(|| {
            info!("New peer clock: {}", peer_id);
            PeerClock {
                filter: KalmanFilter::new(),
                offset: 0.0,
                rtt: 0.0,
                last_update: Instant::now(),
                sample_count: 0,
                drift_ppm: 0.0,
            }
        });
        
        // Update Kalman filter with new sample
        let filtered_offset = peer.filter.update(sample.offset, sample.rtt);
        
        // Calculate drift if we have enough samples
        if peer.sample_count > 10 {
            let time_diff = peer.last_update.elapsed().as_secs_f64();
            let offset_diff = filtered_offset - peer.offset;
            peer.drift_ppm = (offset_diff / time_diff) * 1e6;
        }
        
        peer.offset = filtered_offset;
        peer.rtt = sample.rtt;
        peer.last_update = Instant::now();
        peer.sample_count += 1;
        
        debug!(
            "Clock update for {}: offset={:.3}ms, rtt={:.3}ms, drift={:.1}ppm",
            peer_id,
            peer.offset * 1000.0,
            peer.rtt * 1000.0,
            peer.drift_ppm
        );
        
        // If this is our master, update our offset
        if self.is_master_peer(&peer_id) {
            *self.master_offset.write() = Some(filtered_offset);
        }
    }
    
    /// Check if a peer is our master
    fn is_master_peer(&self, _peer_id: &Uuid) -> bool {
        // TODO: Implement master selection logic
        false
    }
    
    /// Remove stale peer entries
    fn cleanup_stale_peers(&self) {
        let mut peers = self.peers.write();
        let stale_threshold = Duration::from_secs(30);
        
        peers.retain(|id, peer| {
            let is_stale = peer.last_update.elapsed() > stale_threshold;
            if is_stale {
                warn!("Removing stale peer clock: {}", id);
            }
            !is_stale
        });
    }
}