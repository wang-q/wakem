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

/// IPC discovery timeout (milliseconds) - quick scan for active instances
pub const IPC_DISCOVERY_TIMEOUT_MS: u64 = 100;

/// IPC idle timeout for short-lived connections (seconds) - for one-shot commands
pub const IPC_IDLE_TIMEOUT_SHORT_SECS: u64 = 30;

/// IPC channel capacity
pub const IPC_CHANNEL_CAPACITY: usize = 100;

/// IPC protocol version (for version negotiation between client/server)
pub const IPC_PROTOCOL_VERSION: u16 = 1;

// ==================== Input Processing Related Constants ====================

/// Input event channel capacity
pub const INPUT_CHANNEL_CAPACITY: usize = 1000;

/// Window event channel capacity
pub const WINDOW_EVENT_CHANNEL_CAPACITY: usize = 100;

/// Input batch processing timeout (microseconds)
pub const INPUT_BATCH_TIMEOUT_MICROS: u64 = 100;

/// Input batch size limit
pub const INPUT_BATCH_SIZE_LIMIT: usize = 50;

// ==================== Window Management Related Constants ====================

/// Window preset apply delay (milliseconds) - wait for window to be fully created
pub const WINDOW_PRESET_APPLY_DELAY_MS: u64 = 500;

/// Delay to wait for tasks to complete during shutdown (milliseconds)
pub const SHUTDOWN_WAIT_DELAY_MS: u64 = 500;

// ==================== Authentication Related Constants ====================

/// Authentication operation timeout (seconds)
pub const AUTH_OPERATION_TIMEOUT_SECS: u64 = 5;

// ==================== Rate Limiting Constants ====================

/// Default maximum connection attempts
pub const RATE_LIMIT_MAX_ATTEMPTS: u32 = 5;

/// Rate limiting time window (seconds)
pub const RATE_LIMIT_WINDOW_SECS: u64 = 60;

// ==================== Wheel Constants ====================

/// Default wheel speed
pub const DEFAULT_WHEEL_SPEED: i32 = 3;

/// Default wheel acceleration multiplier
pub const DEFAULT_ACCELERATION_MULTIPLIER: f32 = 2.0;

/// Default wheel step value
pub const DEFAULT_WHEEL_STEP: i32 = 1;

// ==================== Retry and Timeout Constants ====================

/// Default retry delay for reconnection attempts (milliseconds)
pub const DEFAULT_RETRY_DELAY_MS: u64 = 500;

/// Maximum reconnection retries for daemon client
pub const MAX_RECONNECT_RETRIES: u32 = 3;

/// Maximum connection retries for initial connection
pub const MAX_CONNECTION_RETRIES: u32 = 10;

/// Short sleep duration for polling (milliseconds)
pub const POLLING_SLEEP_MS: u64 = 10;

/// Medium sleep duration for polling (milliseconds)
pub const MEDIUM_POLLING_SLEEP_MS: u64 = 100;
