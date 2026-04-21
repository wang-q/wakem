# AGENTS.md

此文件为 AI 助手在处理本仓库代码时提供指南与上下文。

## 项目概览

**当前状态**: 活跃开发中 | **主要语言**: Rust | **版本**: 0.1.0 | **平台**: Windows; macOS/Linux(wayland) 将在后续迁移。

**语言约定**: 为了便于指导，本文件 (`AGENTS.md`) 使用中文编写，且**与用户交流时请使用中文**。但项目代码中的
**所有文档注释 (doc comments)**、**行内注释**以及**提交信息**必须使用**英文**。

`wakem` (Window Adjust, Keyboard Enhance, and Mouse) 是一个跨平台的输入增强和窗口管理工具，使用 Rust 语言实现。

### 核心特性

- **键盘增强**: 键位重映射、修饰键自定义、宏录制与播放
- **鼠标增强**: 滚轮加速/减速、鼠标按键映射、水平滚动支持
- **窗口管理**: 窗口移动/调整大小/最大化/最小化、预设布局、自动应用规则
- **进程启动**: 快捷键启动应用程序
- **守护进程模式**: 后台运行，通过 IPC 接受客户端命令
- **配置系统**: TOML 格式配置文件，支持热重载和多实例
- **系统托盘**: Windows 系统托盘图标集成

## 项目结构

```
src/
├── main.rs              # CLI 入口和命令行参数解析
├── lib.rs               # 库根模块，导出公共 API
├── constants.rs         # 全局常量定义（IPC 端口、超时时间等）
│
├── cli.rs               # 命令行接口定义 (clap)
├── client.rs            # IPC 客户端，用于向守护进程发送命令
├── daemon.rs            # 守护进程核心逻辑（主循环、事件处理）
├── config.rs            # 配置文件解析、验证、键名映射、通配符匹配
├── shutdown.rs          # 优雅关闭信号管理
├── window.rs            # Windows 消息窗口封装（托盘图标等）
│
├── ipc/                 # 进程间通信模块
│   ├── mod.rs           # IPC 消息序列化/反序列化
│   ├── server.rs        # IPC 服务端（监听连接、消息分发）
│   ├── client.rs        # IPC 客户端底层实现
│   ├── auth.rs          # HMAC-SHA256 认证机制
│   ├── discovery.rs     # 实例发现（广播/扫描）
│   ├── security.rs      # 安全策略（IP 白名单）
│   └── rate_limiter.rs  # 连接速率限制
│
├── platform/            # 平台抽象层
│   └── windows/         # Windows 平台实现
│       ├── mod.rs       # 平台模块入口
│       ├── input.rs    # 输入设备抽象 trait
│       ├── output_device.rs  # 输出设备抽象 trait
│       ├── input_device.rs   # Raw Input 设备实现（含 Mock）
│       ├── launcher.rs  # 应用程序启动器
│       ├── context.rs   # 窗口上下文信息获取
│       ├── tray.rs      # 系统托盘图标
│       ├── tray_api.rs  # 托盘 API 封装
│       ├── window_api.rs      # Win32 API 封装
│       ├── window_manager.rs  # 窗口管理器（移动/调整/状态）
│       ├── window_event_hook.rs  # 窗口事件钩子
│       └── window_preset.rs     # 窗口预设匹配和应用
│
├── runtime/             # 运行时逻辑层
│   ├── mod.rs           # 运行时模块入口
│   ├── mapper.rs        # 输入事件到动作的映射引擎
│   ├── layer_manager.rs # 层管理系统（层激活/切换/映射）
│   └── macro_player.rs  # 宏录制与播放
│
└── types/               # 核心数据类型定义
    ├── mod.rs           # 类型模块入口
    ├── action.rs        # 动作类型（Key/Mouse/Window/Launch/System/Delay/Sequence）
    ├── input.rs         # 输入事件类型（KeyEvent/MouseEvent/InputEvent）
    ├── key_codes.rs     # 扫描码/虚拟键码定义和转换
    ├── layer.rs         # 层定义（Layer/LayerStack）
    ├── mapping.rs       # 映射规则（MappingRule/Trigger/ContextCondition）
    └── macros.rs        # 宏相关类型（MacroStep/MacroRecorder）

```

### 模块访问路径

- 核心类型通过 `wakem::types::*` 访问（如 `wakem::types::InputEvent`, `wakem::types::Action`）
- 配置通过 `wakem::config::Config` 访问
- 守护进程通过 `wakem::daemon::WakemServer` 访问
- IPC 客户端通过 `wakem::client::DaemonClient` 访问
- 平台特定功能通过 `wakem::platform::windows::*` 访问
- 全局常量通过 `wakem::constants::*` 访问
- 测试分为：
  - 单元测试：在各源文件底部的 `#[cfg(test)]` 模块中
  - 集成测试：`tests/integration_tests.rs`
  - 属性测试：`tests/property_tests.rs`（使用 proptest）

## 构建命令

### 构建

```bash
# 开发构建
cargo build

# 发布构建 (高性能，启用 LTO)
cargo build --release

# 运行所有测试
cargo test

# 仅运行单元测试
cargo test --lib
```

### 代码质量检查

```bash
# 格式化代码
cargo fmt

# Clippy 静态分析
cargo clippy

# Clippy 严格模式（建议开发时使用）
cargo clippy -- -D warnings
```

## 依赖说明

### 核心依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `windows` | 0.52 | Windows API 绑定（UI、输入、窗口管理等） |
| `tokio` | 1.35 | 异步运行时（full features） |
| `serde` / `toml` | 1.0 / 0.8 | 序列化/反序列化（配置文件） |
| `tracing` | 0.1 | 结构化日志 |
| `clap` | 4.4 | 命令行参数解析 |
| `parking_lot` | 0.12 | 高性能同步原语（Mutex/RwLock） |

### 功能性依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `hmac` / `sha2` | 0.12 / 0.10 | IPC 通信的 HMAC-SHA256 认证 |
| `rand` | 0.8 | 认证密钥生成 |
| `chrono` | 0.4 | 时间戳处理 |
| `indexmap` | 2.0 | 有序 HashMap（保持配置顺序） |
| `once_cell` | 1.19 | 延迟初始化全局单例 |
| `async-trait` | 0.1 | 异步 trait 支持 |

### 开发依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `proptest` | 1.4 | 属性测试（随机数据生成、边缘情况发现） |

## 代码规范

- 使用 `cargo fmt` 格式化代码。
- 使用 `cargo clippy` 检查潜在问题，修复所有 warnings。
- 优先使用标准库和项目中已引入的 crate。
- 保持代码简洁，注重性能（特别是输入处理路径）。
- 所有公共 API 必须包含文档注释（英文）。
- 使用 `tracing` 宏进行日志记录（`info!`, `debug!`, `warn!`, `error!`），避免使用 `println!` 或 `eprintln!`。

## 测试规范

项目采用**多层次测试策略**：

- **单元测试**: 所有核心功能必须在源文件底部的 `#[cfg(test)]` 模块中编写单元测试
- **集成测试**: 跨模块交互在 `tests/integration_tests.rs` 中测试
- **属性测试**: 使用 proptest 在 `tests/property_tests.rs` 中发现边缘情况
- **Mock 测试**: 平台相关功能提供 Mock 实现，便于单元测试

## 架构设计要点

### 分层架构

```
┌─────────────────────────────────────┐
│           main.rs (CLI)             │  用户入口
├─────────────────────────────────────┤
│         client.rs (IPC Client)      │  命令发送
├─────────────────┬───────────────────┤
│  daemon.rs      │  ipc/server.rs   │  核心逻辑  │  通信层
├─────────────────┼───────────────────┤
│  runtime/       │  types/          │  业务逻辑  │  数据模型
│  ├─ mapper      │  ├─ action       │
│  ├─ layer_mgr   │  ├─ input        │
│  └─ macro_player│  └─ mapping      │
├─────────────────┴───────────────────┤
│      platform/windows/              │  平台实现
│  ├─ input_device (Raw Input)       │
│  ├─ output_device (SendInput)      │
│  ├─ window_manager                 │
│  └─ tray                           │
└─────────────────────────────────────┘
```

## 开发者文档规范

`docs/developer.md` 是供项目开发者参考的内部指南，不要包含在最终生成的用户文档（mdBook 站点）中。

### 文档格式

* **语言**: 使用**中文**编写。
* **格式**: 避免过多的加粗 (Bold) 或强调格式，以保持在纯文本编辑器中的可读性。
* **结构**: 使用清晰的标题层级组织内容
* **代码示例**: 包含完整的命令和代码片段，便于复制使用
* **表格**: 使用表格展示比较信息，提高可读性

### 维护要求

* 定期更新文档，反映项目的最新状态
* 保持文档与代码的一致性
* 新增功能或架构变更后及时更新相关文档
