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

/// IPC request/response timeout (seconds)
/// Separate from connection timeout as request processing may take longer
/// (e.g., config reload, macro playback)
pub const IPC_REQUEST_TIMEOUT_SECS: u64 = 10;

/// IPC discovery timeout (milliseconds) - quick scan for active instances
pub const IPC_DISCOVERY_TIMEOUT_MS: u64 = 100;

/// IPC idle timeout for short-lived connections (seconds) - for one-shot commands
/// like status, reload, save, etc. These connections should complete quickly.
pub const IPC_IDLE_TIMEOUT_SHORT_SECS: u64 = 30;

/// IPC idle timeout for long-lived connections (seconds) - for tray clients
/// that maintain a persistent connection for status updates and notifications.
pub const IPC_IDLE_TIMEOUT_LONG_SECS: u64 = 600;

/// IPC channel capacity
pub const IPC_CHANNEL_CAPACITY: usize = 100;

// ==================== Input Processing Related Constants ====================

/// Input event channel capacity
pub const INPUT_CHANNEL_CAPACITY: usize = 1000;

/// Input batch processing timeout (microseconds)
pub const INPUT_BATCH_TIMEOUT_MICROS: u64 = 100;

/// Input batch size limit
pub const INPUT_BATCH_SIZE_LIMIT: usize = 50;

// ==================== Window Management Related Constants ====================

/// Delay to wait for tasks to complete during shutdown (milliseconds)
pub const SHUTDOWN_WAIT_DELAY_MS: u64 = 500;

/// Delay before auto-applying window preset after window activation (milliseconds)
pub const WINDOW_PRESET_APPLY_DELAY_MS: u64 = 100;

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

/// Delay between hyper key press and modifier injection (milliseconds)
pub const HYPER_KEY_INJECTION_DELAY_MS: u64 = 10;

// ==================== Retry and Timeout Constants ====================

/// Default retry delay for reconnection attempts (milliseconds)
pub const DEFAULT_RETRY_DELAY_MS: u64 = 500;

/// Maximum reconnection retries for daemon client
pub const MAX_RECONNECT_RETRIES: u32 = 3;

/// Maximum connection retries for initial connection
pub const MAX_CONNECTION_RETRIES: u32 = 10;

/// Daemon shutdown timeout (seconds)
pub const DAEMON_SHUTDOWN_TIMEOUT_SECS: u64 = 10;
