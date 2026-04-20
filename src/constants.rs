/// wakem 全局常量定义
///
/// 集中管理所有配置常量，避免魔法数字，提高代码可维护性

// ==================== IPC 相关常量 ====================

/// IPC 基础端口
pub const IPC_BASE_PORT: u16 = 57427;

/// IPC 最大消息大小 (1MB)
pub const IPC_MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// IPC 连接超时时间（秒）
pub const IPC_CONNECTION_TIMEOUT_SECS: u64 = 5;

/// IPC 空闲超时时间（秒）- 2分钟，平衡资源使用和用户体验
pub const IPC_IDLE_TIMEOUT_SECS: u64 = 120;

/// IPC 通道容量
pub const IPC_CHANNEL_CAPACITY: usize = 100;

// ==================== 输入处理相关常量 ====================

/// 输入事件通道容量
pub const INPUT_CHANNEL_CAPACITY: usize = 1000;

/// 窗口事件通道容量
pub const WINDOW_EVENT_CHANNEL_CAPACITY: usize = 100;

// ==================== 窗口管理相关常量 ====================

/// 窗口预设应用延迟（毫秒）- 等待窗口完全创建
pub const WINDOW_PRESET_APPLY_DELAY_MS: u64 = 500;

/// 关闭时等待任务完成的延迟（毫秒）
pub const SHUTDOWN_WAIT_DELAY_MS: u64 = 500;

// ==================== 通配符匹配相关常量 ====================

/// 通配符匹配最大输入长度（防止内存问题）
pub const WILDCARD_MAX_INPUT_SIZE: usize = 1024;

// ==================== 认证相关常量 ====================

/// 挑战长度（字节）
pub const AUTH_CHALLENGE_SIZE: usize = 32;

/// 响应长度（字节，HMAC-SHA256 输出）
pub const AUTH_RESPONSE_SIZE: usize = 32;

/// 认证操作超时时间（秒）
pub const AUTH_OPERATION_TIMEOUT_SECS: u64 = 5;

// ==================== 速率限制相关常量 ====================

/// 默认最大连接尝试次数
pub const RATE_LIMIT_MAX_ATTEMPTS: u32 = 5;

/// 速率限制时间窗口（秒）
pub const RATE_LIMIT_WINDOW_SECS: u64 = 60;

// ==================== 滚轮相关常量 ====================

/// WHEEL_DELTA 标准值
pub const WHEEL_DELTA: i32 = 120;

/// 默认滚轮速度
pub const DEFAULT_WHEEL_SPEED: i32 = 3;

/// 默认滚轮加速倍数
pub const DEFAULT_ACCELERATION_MULTIPLIER: f32 = 2.0;

/// 默认滚轮步进值
pub const DEFAULT_WHEEL_STEP: i32 = 1;
