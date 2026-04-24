# AGENTS.md

此文件为 AI 助手在处理本仓库代码时提供指南与上下文。

## 项目概览

**当前状态**: 活跃开发中 | **主要语言**: Rust | **版本**: 0.1.1 | **平台**: Windows (完整支持), macOS (开发中), Linux(Wayland) 将在后续迁移.

**语言约定**: 为了便于指导，本文件 (`AGENTS.md`) 使用中文编写，且**与用户交流时请使用中文**。但项目代码中的
**所有文档注释 (doc comments)**、**行内注释**以及**提交信息**必须使用**英文**。

`wakem` (Window Adjust, Keyboard Enhance, and Mouse) 是一个跨平台的输入增强和窗口管理工具，使用 Rust 语言实现。

### 核心特性

- **键盘增强**: 键位重映射、修饰键自定义、宏录制与播放
- **鼠标增强**: 滚轮加速/减速、水平滚动、音量/亮度控制
- **窗口管理**: 窗口移动/调整大小/最大化/最小化、预设布局、自动应用规则、跨显示器移动
- **进程启动**: 快捷键启动应用程序（支持参数）
- **守护进程模式**: 后台运行，通过 IPC 接受客户端命令
- **配置系统**: TOML 格式配置文件，支持热重载和多实例
- **系统托盘**: Windows/macOS 系统托盘图标集成
- **多实例支持**: 同时运行多个独立配置的实例

## 项目结构

```
src/
├── main.rs              # CLI 入口和命令行参数解析
├── lib.rs               # 库根模块，导出公共 API
├── constants.rs         # 全局常量定义（IPC 端口、超时时间等）
├── cli.rs               # 命令行接口定义 (clap)
├── client.rs            # IPC 客户端，用于向守护进程发送命令
├── daemon.rs            # 守护进程核心逻辑（主循环、事件处理）
├── config.rs            # 配置文件解析、验证、键名映射、通配符匹配
├── shutdown.rs          # 优雅关闭信号管理
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
│   ├── mod.rs           # 平台模块入口和条件编译
│   ├── traits.rs        # 平台无关的 trait 定义
│   ├── mock.rs          # Mock 实现（用于测试）
│   ├── output_helpers.rs # 输出设备辅助函数
│   ├── windows/         # Windows 平台实现
│   │   ├── mod.rs       # Windows 平台模块入口
│   │   ├── input.rs     # 输入设备抽象 trait
│   │   ├── input_device.rs   # Raw Input 设备实现
│   │   ├── output_device.rs  # SendInput 输出实现
│   │   ├── launcher.rs  # 应用程序启动器
│   │   ├── context.rs   # 窗口上下文信息获取
│   │   ├── tray.rs      # 系统托盘图标
│   │   ├── window_api.rs      # Win32 API 封装
│   │   ├── window_manager.rs  # 窗口管理器（移动/调整/状态）
│   │   ├── window_event_hook.rs  # 窗口事件钩子
│   │   └── window_preset.rs     # 窗口预设匹配和应用
│   └── macos/           # macOS 平台实现
│       ├── mod.rs       # macOS 平台模块入口
│       ├── input.rs     # 输入设备抽象 trait
│       ├── input_device.rs   # CGEvent 输入设备实现
│       ├── output_device.rs  # CGEvent 输出实现
│       ├── launcher.rs  # 应用程序启动器
│       ├── context.rs   # 窗口上下文信息获取
│       ├── tray.rs      # 系统托盘图标
│       ├── window_api.rs      # Cocoa/Accessibility API 封装
│       ├── window_manager.rs  # 窗口管理器
│       ├── window_event_hook.rs  # 窗口事件监听
│       ├── window_preset.rs     # 窗口预设匹配和应用
│       └── native_api/  # macOS 原生 API 封装
│           ├── mod.rs           # 原生 API 模块入口
│           ├── ax_element.rs    # Accessibility API 封装
│           ├── cg_window.rs     # Core Graphics 窗口 API
│           ├── coordinate.rs    # 坐标转换
│           ├── core_audio.rs    # 音频控制 API
│           ├── display.rs       # 显示器信息 API
│           ├── notification.rs  # 通知中心 API
│           └── ns_workspace.rs  # NSWorkspace API
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
    ├── macros.rs        # 宏相关类型（MacroStep/MacroRecorder）
    └── time_source.rs   # 时间源抽象（用于测试）

tests/                   # 测试目录（1 层子目录结构）
├── types/                  # 类型系统测试 (6 files)
│   ├── basic.rs           # 基础类型创建和匹配
│   ├── comprehensive.rs   # 完整边界条件覆盖
│   ├── action.rs          # Action 类型变体
│   ├── input_event.rs     # KeyEvent/MouseEvent
│   ├── mapping_rule.rs    # MappingRule 触发器
│   └── layer.rs           # Layer/LayerMode
│
├── runtime/                # 运行时逻辑测试 (3 files)
│   ├── mapper.rs          # KeyMapper 映射引擎
│   ├── mapper_full.rs     # KeyMapper 深度测试
│   └── layer_manager.rs   # LayerManager 层管理
│
├── config/                 # 配置系统测试 (3 files)
│   ├── parser.rs          # 配置解析基础
│   ├── comprehensive.rs    # 配置完整场景
│   └── edge_cases.rs      # 配置边界条件
│
├── core/                   # 核心功能测试 (5 files)
│   ├── daemon.rs          # ServerState 守护进程逻辑
│   ├── cli.rs             # CLI 参数解析
│   ├── client.rs          # DaemonClient 通信层
│   ├── ipc.rs             # IPC 消息序列化
│   └── security.rs        # 安全策略（IP 白名单等）
│
├── window/                 # 窗口管理测试 (2 files)
│   ├── calc.rs            # 窗口位置/大小计算算法
│   └── manager.rs         # 窗口管理器逻辑
│
├── integration/            # 集成测试 (3 files)
│   ├── core.rs            # Config + Runtime + Types 集成
│   ├── edge_cases.rs      # 跨模块边界情况
│   └── ipc.rs             # IPC 集成测试
│
├── property/               # 属性测试 (proptest) (4 files)
│   ├── config.rs          # 通配符匹配、键名解析
│   ├── config.regressions # proptest 回归种子
│   ├── macos_keycode.rs   # macOS 键码映射 [macOS only]
│   └── macos_keycode.regressions  # macOS 回归种子 [macOS only]
│
├── windows/                # Windows 平台特定测试 (3 files)
│   ├── e2e.rs             # Windows 端到端测试（操作真实窗口，需手动运行）
│   ├── tray.rs            # Windows 系统托盘
│   └── specific.rs        # MonitorInfo, WindowFrame
│
└── macos/                  # macOS 平台特定测试 (1 file)
    └── integration.rs     # macOS 集成测试

benches/                  # 性能基准测试 (cargo bench)
├── basic_benchmarks.rs    # 跨平台基准测试（8 个 benchmark）
└── macos/
    └── macos_bench.rs     # macOS 专用基准 [macOS only]

examples/                # 配置示例
├── minimal.toml         # 最小配置示例
├── navigation_layer.toml # 导航层配置示例
├── window_manager.toml   # 窗口管理配置示例
└── test_config.toml      # 测试配置

docs/                    # 文档
├── config.md            # 配置指南
├── developer.md         # 开发者文档
├── macros.md            # 宏系统文档
└── keys.md              # 键名参考
```

### 模块访问路径

- 核心类型通过 `wakem::types::*` 访问（如 `wakem::types::InputEvent`, `wakem::types::Action`）
- 配置通过 `wakem::config::Config` 访问
- 守护进程通过 `wakem::daemon::WakemServer` 访问
- IPC 客户端通过 `wakem::client::DaemonClient` 访问
- 平台特定功能通过 `wakem::platform::*` 访问
- 全局常量通过 `wakem::constants::*` 访问

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

# 运行特定测试（按目录）
cargo test --test types::basic
cargo test --test core::daemon

# 运行性能基准测试
cargo bench
cargo bench --bench basic_benchmarks  # 仅跨平台基准 [macOS: cargo bench --bench macos_bench]
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
| `serde` / `serde_json` / `toml` | 1.0 / 1.0 / 0.8 | 序列化/反序列化（配置文件） |
| `tokio` | 1.35 | 异步运行时（full features） |
| `tracing` / `tracing-subscriber` | 0.1 / 0.3 | 结构化日志 |
| `clap` | 4.4 | 命令行参数解析 |
| `parking_lot` | 0.12 | 高性能同步原语（Mutex/RwLock） |
| `keyboard-codes` | 0.3 | 跨平台键码映射 |
| `anyhow` / `thiserror` | 1.0 | 错误处理 |
| `indexmap` | 2.0 | 有序 HashMap（保持配置顺序） |
| `once_cell` / `lazy_static` | 1.19 / 1.4 | 延迟初始化全局单例 |
| `dirs` | 5 | 跨平台目录路径获取 |
| `chrono` | 0.4 | 时间戳处理 |
| `async-trait` | 0.1 | 异步 trait 支持 |

### 平台特定依赖

#### Windows

| Crate | 版本 | 用途 |
|-------|------|------|
| `windows` | 0.61 | Windows API 绑定（UI、输入、窗口管理等） |
| `windows-core` | 0.61 | Windows 核心类型 |

#### macOS

| Crate | 版本 | 用途 |
|-------|------|------|
| `core-graphics` | 0.24 | Core Graphics API（输入/窗口） |
| `core-foundation` | 0.10 | Core Foundation 基础类型 |
| `cocoa` | 0.26 | Cocoa UI 框架 |
| `objc` | 0.2 | Objective-C 运行时绑定 |
| `accessibility` | 0.2 | Accessibility API |
| `libc` | 0.2 | 原生 API 调用（pid_t, c_char, proc_pidpath） |

### 功能性依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `hmac` / `sha2` | 0.12 / 0.10 | IPC 通信的 HMAC-SHA256 认证 |
| `rand` | 0.8 | 认证密钥生成 |
| `regex` | 1.10 | 正则表达式匹配 |

### 开发依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `proptest` | 1.4 | 属性测试（随机数据生成、边缘情况发现） |
| `criterion` | 0.5 | 性能基准测试 |

## 代码规范

- 使用 `cargo fmt` 格式化代码。
- 使用 `cargo clippy` 检查潜在问题，修复所有 warnings。
- 优先使用标准库和项目中已引入的 crate。
- 保持代码简洁，注重性能（特别是输入处理路径）。
- 所有公共 API 必须包含文档注释（英文）。
- 使用 `tracing` 宏进行日志记录（`info!`, `debug!`, `warn!`, `error!`），避免使用 `println!` 或 `eprintln!`。

## 测试规范

项目采用**多层次测试策略**，按功能域组织在 1 层子目录中：

- **单元测试**: 所有核心功能必须在源文件底部的 `#[cfg(test)]` 模块中编写单元测试
- **集成测试**: 跨模块交互在 `tests/integration/` 目录中测试
- **属性测试**: 使用 proptest 在 `tests/property/` 中发现边缘情况
- **Mock 测试**: 平台相关功能提供 Mock 实现，便于单元测试
- **性能基准**: 使用 Criterion 在 `benches/` 中进行专业基准测试

### 测试目录结构

```
tests/
├── types/          # 类型系统 (basic, comprehensive, action, input_event, mapping_rule, layer)
├── runtime/        # 运行时逻辑 (mapper, mapper_full, layer_manager)
├── config/         # 配置系统 (parser, comprehensive, edge_cases)
├── core/           # 核心功能 (daemon, cli, client, ipc, security)
├── window/         # 窗口管理 (calc, manager)
├── integration/    # 集成测试 (core, edge_cases, ipc)
├── property/       # 属性测试 (config, macos_keycode) [proptest]
├── windows/        # Windows 平台特定 (e2e, tray, specific)
└── macos/          # macOS 平台特定 (integration)

benches/
├── basic_benchmarks.rs  # 跨平台基准
└── macos/
    └── macos_bench.rs   # macOS 基准
```

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
│      platform/                      │  平台实现
│  ├─ traits.rs (平台无关接口)        │
│  ├─ windows/                        │
│  │   ├─ input_device (Raw Input)   │
│  │   ├─ output_device (SendInput)  │
│  │   └─ window_manager             │
│  └─ macos/                          │
│      ├─ input_device (CGEvent)     │
│      ├─ output_device (CGEvent)    │
│      └─ window_manager             │
└─────────────────────────────────────┘
```

### 平台抽象

- `platform::traits.rs` 定义了跨平台的 trait 接口
- `platform::mock.rs` 提供了用于测试的 Mock 实现
- Windows 和 macOS 分别实现了这些 trait
- 使用条件编译 (`#[cfg(target_os = "...")]`) 选择平台实现

### 配置键名支持

配置文件支持多种修饰键名称（不区分大小写）：

- **Ctrl**: `ctrl`, `control`
- **Alt**: `alt`
- **Shift**: `shift`
- **Meta/Win**: `win`, `meta`, `command`, `cmd`

这使得配置在不同平台上更具通用性。

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
