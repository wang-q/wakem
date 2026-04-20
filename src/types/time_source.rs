//! Time source abstraction for controlling time during tests

/// Time source trait
pub trait TimeSource {
    /// Get current timestamp (milliseconds)
    fn now_ms(&self) -> u64;
    /// Get current timestamp (microseconds)
    fn now_us(&self) -> u64;
}

/// System time source (real time)
pub struct SystemTimeSource;

impl SystemTimeSource {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemTimeSource {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeSource for SystemTimeSource {
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn now_us(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }
}

/// Mock time source for testing
#[cfg(test)]
pub struct MockTimeSource {
    current_time_ms: std::cell::RefCell<u64>,
}

#[cfg(test)]
impl MockTimeSource {
    pub fn new() -> Self {
        Self {
            current_time_ms: std::cell::RefCell::new(0),
        }
    }

    pub fn with_start_time(start_ms: u64) -> Self {
        Self {
            current_time_ms: std::cell::RefCell::new(start_ms),
        }
    }

    /// 推进时间（毫秒）
    pub fn advance_ms(&self, ms: u64) {
        *self.current_time_ms.borrow_mut() += ms;
    }

    /// 推进时间（微秒）
    pub fn advance_us(&self, us: u64) {
        *self.current_time_ms.borrow_mut() += us / 1000;
    }

    /// 设置当前时间
    pub fn set_time(&self, ms: u64) {
        *self.current_time_ms.borrow_mut() = ms;
    }

    /// 获取当前时间
    pub fn get_time(&self) -> u64 {
        *self.current_time_ms.borrow()
    }
}

#[cfg(test)]
impl TimeSource for MockTimeSource {
    fn now_ms(&self) -> u64 {
        *self.current_time_ms.borrow()
    }

    fn now_us(&self) -> u64 {
        *self.current_time_ms.borrow() * 1000
    }
}

#[cfg(test)]
impl Default for MockTimeSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_time_source() {
        let time_source = SystemTimeSource::new();
        let t1 = time_source.now_ms();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let t2 = time_source.now_ms();
        assert!(t2 > t1);
    }

    #[test]
    fn test_mock_time_source_creation() {
        let time_source = MockTimeSource::new();
        assert_eq!(time_source.now_ms(), 0);
        assert_eq!(time_source.now_us(), 0);
    }

    #[test]
    fn test_mock_time_source_advance() {
        let time_source = MockTimeSource::new();

        time_source.advance_ms(100);
        assert_eq!(time_source.now_ms(), 100);

        time_source.advance_ms(50);
        assert_eq!(time_source.now_ms(), 150);
    }

    #[test]
    fn test_mock_time_source_set_time() {
        let time_source = MockTimeSource::new();

        time_source.set_time(1000);
        assert_eq!(time_source.now_ms(), 1000);

        time_source.set_time(500);
        assert_eq!(time_source.now_ms(), 500);
    }

    #[test]
    fn test_mock_time_source_with_start_time() {
        let time_source = MockTimeSource::with_start_time(1000);
        assert_eq!(time_source.now_ms(), 1000);
    }

    #[test]
    fn test_mock_time_source_advance_us() {
        let time_source = MockTimeSource::new();

        time_source.advance_us(5000); // 5ms
        assert_eq!(time_source.now_ms(), 5);

        time_source.advance_us(2500); // 2.5ms -> 2ms (truncated)
        assert_eq!(time_source.now_ms(), 7);
    }
}
