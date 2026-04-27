use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use crate::constants::{RATE_LIMIT_MAX_ATTEMPTS, RATE_LIMIT_WINDOW_SECS};

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
    attempts: HashMap<IpAddr, Vec<Instant>>,
    /// Maximum allowed attempts
    pub max_attempts: u32,
    /// Time window (seconds)
    pub window_seconds: u64,
}

impl ConnectionLimiter {
    /// Create a new rate limiter
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
    pub fn check_rate_limit(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(self.window_seconds);

        if self.attempts.len() >= MAX_TRACKED_IPS {
            self.cleanup_oldest_entries(MAX_TRACKED_IPS - CLEANUP_THRESHOLD);
        }

        let attempt_times = self.attempts.entry(ip).or_default();
        attempt_times.retain(|&time| now.duration_since(time) < window);

        let allowed = attempt_times.len() < self.max_attempts as usize;
        if allowed {
            attempt_times.push(now);
        }

        allowed
    }

    fn cleanup_oldest_entries(&mut self, count: usize) {
        let mut ip_ages: Vec<(IpAddr, Instant)> = self
            .attempts
            .iter()
            .filter_map(|(ip, times)| times.first().copied().map(|t| (*ip, t)))
            .collect();

        let count = count.min(ip_ages.len());
        if count > 0 {
            ip_ages.select_nth_unstable_by(count - 1, |a, b| a.1.cmp(&b.1));
            for (ip, _) in ip_ages.iter().take(count) {
                self.attempts.remove(ip);
            }
        }
    }

    /// Reset limit count for specified IP
    #[cfg(test)]
    pub fn reset(&mut self, ip: &IpAddr) {
        self.attempts.remove(ip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_basic_rate_limiting() {
        let mut limiter = ConnectionLimiter::new(3, 60);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
        assert!(!limiter.check_rate_limit(ip));
    }

    #[test]
    fn test_different_ips() {
        let mut limiter = ConnectionLimiter::new(2, 60);
        let ip1 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));

        assert!(limiter.check_rate_limit(ip1));
        assert!(limiter.check_rate_limit(ip1));
        assert!(!limiter.check_rate_limit(ip1));

        assert!(limiter.check_rate_limit(ip2));
        assert!(limiter.check_rate_limit(ip2));
    }

    #[test]
    fn test_reset() {
        let mut limiter = ConnectionLimiter::new(2, 60);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
        assert!(!limiter.check_rate_limit(ip));

        limiter.reset(&ip);
        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
    }

    #[test]
    fn test_default_config() {
        let limiter = ConnectionLimiter::with_defaults();
        assert_eq!(limiter.max_attempts, RATE_LIMIT_MAX_ATTEMPTS);
        assert_eq!(limiter.window_seconds, RATE_LIMIT_WINDOW_SECS);
    }
}
