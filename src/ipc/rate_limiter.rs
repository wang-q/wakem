use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Connection rate limiter
///
/// Used to prevent brute force and denial of service attacks
/// Features:
/// - IP-based rate limiting
/// - Configurable max attempts and time window
/// - Automatic cleanup of expired records
pub struct ConnectionLimiter {
    /// Attempt records for each IP
    attempts: HashMap<IpAddr, Vec<Instant>>,
    /// Maximum allowed attempts
    max_attempts: u32,
    /// Time window (seconds)
    window_seconds: u64,
}

impl ConnectionLimiter {
    /// Create a new rate limiter
    ///
    /// # Parameters
    /// * `max_attempts` - Maximum allowed attempts within the time window
    /// * `window_seconds` - Time window size (seconds)
    pub fn new(max_attempts: u32, window_seconds: u64) -> Self {
        Self {
            attempts: HashMap::new(),
            max_attempts,
            window_seconds,
        }
    }

    /// Create rate limiter with default configuration
    ///
    /// Defaults:
    /// - Max attempts: 5
    /// - Time window: 60 seconds
    pub fn with_defaults() -> Self {
        Self::new(5, 60)
    }

    /// Check if connection is allowed
    ///
    /// # Returns
    /// * `true` - Connection allowed
    /// * `false` - Rate limit exceeded, connection denied
    pub fn check_rate_limit(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(self.window_seconds);

        // Get or create record for this IP
        let attempt_times = self.attempts.entry(ip).or_default();

        // Cleanup expired records
        attempt_times.retain(|&time| now.duration_since(time) < window);

        // Check if limit exceeded
        if attempt_times.len() >= self.max_attempts as usize {
            return false;
        }

        // Record this attempt
        attempt_times.push(now);
        true
    }

    /// Reset limit count for specified IP
    pub fn reset(&mut self, ip: &IpAddr) {
        self.attempts.remove(ip);
    }

    /// Clear all records
    pub fn clear(&mut self) {
        self.attempts.clear();
    }

    /// Get current number of tracked IPs
    pub fn tracked_count(&self) -> usize {
        self.attempts.len()
    }
}

impl Default for ConnectionLimiter {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_basic_rate_limiting() {
        let mut limiter = ConnectionLimiter::new(3, 60); // 3 times/60 seconds
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // First 3 attempts should succeed
        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));

        // 4th attempt should fail
        assert!(!limiter.check_rate_limit(ip));
    }

    #[test]
    fn test_different_ips() {
        let mut limiter = ConnectionLimiter::new(2, 60);
        let ip1 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));

        // Each IP has independent limits
        assert!(limiter.check_rate_limit(ip1));
        assert!(limiter.check_rate_limit(ip1));
        assert!(!limiter.check_rate_limit(ip1)); // ip1 reached limit

        // ip2 is not affected
        assert!(limiter.check_rate_limit(ip2));
        assert!(limiter.check_rate_limit(ip2));
    }

    #[test]
    fn test_reset() {
        let mut limiter = ConnectionLimiter::new(2, 60);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // Use up quota
        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
        assert!(!limiter.check_rate_limit(ip));

        // Should be usable again after reset
        limiter.reset(&ip);
        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
    }

    #[test]
    fn test_ipv6_support() {
        let mut limiter = ConnectionLimiter::with_defaults();
        let ip = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));

        assert!(limiter.check_rate_limit(ip));
    }

    #[test]
    fn test_default_config() {
        let limiter = ConnectionLimiter::default();
        assert_eq!(limiter.max_attempts, 5);
        assert_eq!(limiter.window_seconds, 60);
    }
}
