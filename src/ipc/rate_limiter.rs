use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// 连接速率限制器
///
/// 用于防止暴力破解和拒绝服务攻击
/// 特性：
/// - 基于 IP 地址的速率限制
/// - 可配置的最大尝试次数和时间窗口
/// - 自动清理过期记录
pub struct ConnectionLimiter {
    /// 每个 IP 的尝试记录
    attempts: HashMap<IpAddr, Vec<Instant>>,
    /// 最大允许的尝试次数
    max_attempts: u32,
    /// 时间窗口（秒）
    window_seconds: u64,
}

impl ConnectionLimiter {
    /// 创建新的速率限制器
    ///
    /// # 参数
    /// * `max_attempts` - 时间窗口内允许的最大尝试次数
    /// * `window_seconds` - 时间窗口大小（秒）
    pub fn new(max_attempts: u32, window_seconds: u64) -> Self {
        Self {
            attempts: HashMap::new(),
            max_attempts,
            window_seconds,
        }
    }

    /// 使用默认配置创建速率限制器
    ///
    /// 默认值：
    /// - 最大尝试次数: 5 次
    /// - 时间窗口: 60 秒
    pub fn with_defaults() -> Self {
        Self::new(5, 60)
    }

    /// 检查是否允许连接
    ///
    /// # 返回值
    /// * `true` - 允许连接
    /// * `false` - 超过速率限制，拒绝连接
    pub fn check_rate_limit(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(self.window_seconds);

        // 获取或创建该 IP 的记录
        let attempt_times = self.attempts.entry(ip).or_insert_with(Vec::new);

        // 清理过期的记录
        attempt_times.retain(|&time| now.duration_since(time) < window);

        // 检查是否超过限制
        if attempt_times.len() >= self.max_attempts as usize {
            return false;
        }

        // 记录此次尝试
        attempt_times.push(now);
        true
    }

    /// 重置指定 IP 的限制计数
    pub fn reset(&mut self, ip: &IpAddr) {
        self.attempts.remove(ip);
    }

    /// 清除所有记录
    pub fn clear(&mut self) {
        self.attempts.clear();
    }

    /// 获取当前跟踪的 IP 数量
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
        let mut limiter = ConnectionLimiter::new(3, 60); // 3次/60秒
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // 前3次应该成功
        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));

        // 第4次应该失败
        assert!(!limiter.check_rate_limit(ip));
    }

    #[test]
    fn test_different_ips() {
        let mut limiter = ConnectionLimiter::new(2, 60);
        let ip1 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));

        // 每个IP有独立的限制
        assert!(limiter.check_rate_limit(ip1));
        assert!(limiter.check_rate_limit(ip1));
        assert!(!limiter.check_rate_limit(ip1)); // ip1 达到限制

        // ip2 不受影响
        assert!(limiter.check_rate_limit(ip2));
        assert!(limiter.check_rate_limit(ip2));
    }

    #[test]
    fn test_reset() {
        let mut limiter = ConnectionLimiter::new(2, 60);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // 用完配额
        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
        assert!(!limiter.check_rate_limit(ip));

        // 重置后应该可以再次使用
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
