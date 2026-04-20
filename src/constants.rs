// wakem global constants definition
//
// Centralized management of all configuration constants to avoid magic numbers and improve code maintainability

// ==================== IPC Related Constants ====================

/// IPC base port
pub const IPC_BASE_PORT: u16 = 57427;

/// IPC max message size (1MB)
pub const IPC_MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// IPC connection timeout (seconds)
pub const IPC_CONNECTION_TIMEOUT_SECS: u64 = 5;

/// IPC idle timeout (seconds) - 2 minutes, balancing resource usage and user experience
pub const IPC_IDLE_TIMEOUT_SECS: u64 = 120;

/// IPC channel capacity
pub const IPC_CHANNEL_CAPACITY: usize = 100;

// ==================== Input Processing Related Constants ====================

/// Input event channel capacity
pub const INPUT_CHANNEL_CAPACITY: usize = 1000;

/// Window event channel capacity
pub const WINDOW_EVENT_CHANNEL_CAPACITY: usize = 100;

// ==================== Window Management Related Constants ====================

/// Window preset apply delay (milliseconds) - wait for window to be fully created
pub const WINDOW_PRESET_APPLY_DELAY_MS: u64 = 500;

/// Delay to wait for tasks to complete during shutdown (milliseconds)
pub const SHUTDOWN_WAIT_DELAY_MS: u64 = 500;

// ==================== Wildcard Matching Related Constants ====================

/// Wildcard matching max input size (prevent memory issues)
pub const WILDCARD_MAX_INPUT_SIZE: usize = 1024;

// ==================== Authentication Related Constants ====================

/// Challenge length (bytes)
pub const AUTH_CHALLENGE_SIZE: usize = 32;

/// Response length (bytes, HMAC-SHA256 output)
pub const AUTH_RESPONSE_SIZE: usize = 32;

/// Authentication operation timeout (seconds)
pub const AUTH_OPERATION_TIMEOUT_SECS: u64 = 5;

// ==================== Rate Limiting Constants ====================

/// Default maximum connection attempts
pub const RATE_LIMIT_MAX_ATTEMPTS: u32 = 5;

/// Rate limiting time window (seconds)
pub const RATE_LIMIT_WINDOW_SECS: u64 = 60;

// ==================== Wheel Constants ====================

/// WHEEL_DELTA standard value
pub const WHEEL_DELTA: i32 = 120;

/// Default wheel speed
pub const DEFAULT_WHEEL_SPEED: i32 = 3;

/// Default wheel acceleration multiplier
pub const DEFAULT_ACCELERATION_MULTIPLIER: f32 = 2.0;

/// Default wheel step value
pub const DEFAULT_WHEEL_STEP: i32 = 1;
