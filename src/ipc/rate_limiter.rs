use crate::constants::{RATE_LIMIT_MAX_ATTEMPTS, RATE_LIMIT_WINDOW_SECS};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Maximum number of IPs to track (prevents memory exhaustion)
const MAX_TRACKED_IPS: usize = 1000;
/// Cleanup threshold - when exceeded, remove oldest entries
const CLEANUP_THRESHOLD: usize = 900;

/// Connection rate limiter
///
/// Used to prevent brute force and denial of service attacks
/// Features:
/// - IP-based rate limiting
/// - Configurable max attempts and time window
/// - Automatic cleanup of expired records
/// - Memory limit protection (max 1000 tracked IPs)
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
    pub fn with_defaults() -> Self {
        Self::new(RATE_LIMIT_MAX_ATTEMPTS, RATE_LIMIT_WINDOW_SECS)
    }

    /// Check if connection is allowed
    ///
    /// # Returns
    /// * `true` - Connection allowed
    /// * `false` - Rate limit exceeded, connection denied
    pub fn check_rate_limit(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(self.window_seconds);

        // Memory protection: cleanup if too many IPs tracked
        if self.attempts.len() >= MAX_TRACKED_IPS {
            self.cleanup_oldest_entries(MAX_TRACKED_IPS - CLEANUP_THRESHOLD);
        }

        // Get or create record for this IP
        let attempt_times = self.attempts.entry(ip).or_default();

        // Cleanup expired records for this IP
        attempt_times.retain(|&time| now.duration_since(time) < window);

        // Check if limit exceeded (before recording this attempt)
        if attempt_times.len() >= self.max_attempts as usize {
            return false;
        }

        // Record this attempt
        attempt_times.push(now);
        true
    }

    /// Cleanup oldest entries when memory limit is reached
    fn cleanup_oldest_entries(&mut self, count: usize) {
        // Find IPs with oldest attempts
        let mut ip_ages: Vec<(IpAddr, Instant)> = self
            .attempts
            .iter()
            .filter_map(|(ip, times)| times.first().copied().map(|t| (*ip, t)))
            .collect();

        // Sort by age (oldest first)
        ip_ages.sort_by(|a, b| a.1.cmp(&b.1));

        // Remove oldest entries
        for (ip, _) in ip_ages.iter().take(count) {
            self.attempts.remove(ip);
        }
    }

    /// Reset limit count for specified IP
    #[allow(dead_code)]
    pub fn reset(&mut self, ip: &IpAddr) {
        self.attempts.remove(ip);
    }

    /// Clear all records
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.attempts.clear();
    }

    /// Get current number of tracked IPs
    #[allow(dead_code)]
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
    use std::net::Ipv4Addr;

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
    fn test_default_config() {
        let limiter = ConnectionLimiter::default();
        assert_eq!(limiter.max_attempts, 5);
        assert_eq!(limiter.window_seconds, 60);
    }
}
