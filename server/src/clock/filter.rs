use nalgebra::{Matrix2, Vector2};

/// Kalman filter for smoothing clock offset measurements
/// 
/// State vector: [offset, drift_rate]
/// Measurement: offset
pub struct KalmanFilter {
    /// State estimate [offset, drift_rate]
    state: Vector2<f64>,
    
    /// Error covariance matrix
    covariance: Matrix2<f64>,
    
    /// Process noise covariance
    process_noise: Matrix2<f64>,
    
    /// Measurement noise variance
    measurement_noise: f64,
    
    /// Last update timestamp
    last_update: Option<f64>,
}

impl KalmanFilter {
    pub fn new() -> Self {
        Self {
            state: Vector2::zeros(),
            covariance: Matrix2::identity() * 1.0,
            process_noise: Matrix2::new(
                1e-6, 0.0,   // offset process noise
                0.0,  1e-8,  // drift process noise
            ),
            measurement_noise: 1e-3, // measurement noise variance
            last_update: None,
        }
    }
    
    /// Update filter with new offset measurement
    pub fn update(&mut self, measured_offset: f64, rtt: f64) -> f64 {
        let current_time = crate::protocol::get_current_time();
        
        // Adjust measurement noise based on RTT (higher RTT = more noise)
        self.measurement_noise = 1e-4 + (rtt * rtt * 0.1).min(0.01);
        
        if let Some(last_time) = self.last_update {
            let dt = current_time - last_time;
            
            // Predict step
            self.predict(dt);
            
            // Update step
            self.correct(measured_offset);
        } else {
            // First measurement - initialize state
            self.state[0] = measured_offset;
            self.state[1] = 0.0;
        }
        
        self.last_update = Some(current_time);
        
        // Return filtered offset
        self.state[0]
    }
    
    /// Predict step of Kalman filter
    fn predict(&mut self, dt: f64) {
        // State transition matrix
        let f = Matrix2::new(
            1.0, dt,   // offset += drift * dt
            0.0, 1.0,  // drift remains constant
        );
        
        // Predict state
        self.state = f * self.state;
        
        // Predict covariance
        self.covariance = f * self.covariance * f.transpose() + self.process_noise * dt;
    }
    
    /// Correction step of Kalman filter
    fn correct(&mut self, measurement: f64) {
        // Measurement matrix (we only measure offset, not drift)
        let h = Vector2::new(1.0, 0.0);
        
        // Innovation (measurement residual)
        let innovation = measurement - h.dot(&self.state);
        
        // Innovation covariance
        let s = h.dot(&(self.covariance * h)) + self.measurement_noise;
        
        // Kalman gain
        let k = self.covariance * h / s;
        
        // Update state
        self.state += k * innovation;
        
        // Update covariance
        let i_minus_kh = Matrix2::identity() - k * h.transpose();
        self.covariance = i_minus_kh * self.covariance;
    }
    
    /// Get current offset estimate
    pub fn offset(&self) -> f64 {
        self.state[0]
    }
    
    /// Get current drift rate estimate (seconds per second)
    pub fn drift_rate(&self) -> f64 {
        self.state[1]
    }
    
    /// Reset the filter
    pub fn reset(&mut self) {
        self.state = Vector2::zeros();
        self.covariance = Matrix2::identity() * 1.0;
        self.last_update = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_kalman_filter_convergence() {
        let mut filter = KalmanFilter::new();
        
        // Simulate measurements with noise around true offset of 0.1
        let true_offset = 0.1;
        let measurements = vec![
            0.095, 0.103, 0.098, 0.102, 0.099,
            0.101, 0.097, 0.100, 0.099, 0.101,
        ];
        
        for (i, &measurement) in measurements.iter().enumerate() {
            let filtered = filter.update(measurement, 0.01);
            
            // Should converge towards true value
            if i > 5 {
                assert!((filtered - true_offset).abs() < 0.005);
            }
        }
    }
    
    #[test]
    fn test_kalman_filter_drift() {
        let mut filter = KalmanFilter::new();
        
        // Simulate linear drift
        let base_offset = 0.1;
        let drift_rate = 0.001; // 1ms per second
        
        for i in 0..20 {
            let time = i as f64;
            let true_offset = base_offset + drift_rate * time;
            let measurement = true_offset + (i as f64 * 0.0001); // Small noise
            
            filter.update(measurement, 0.01);
        }
        
        // Filter should estimate drift rate
        assert!((filter.drift_rate() - drift_rate).abs() < 0.0005);
    }
}