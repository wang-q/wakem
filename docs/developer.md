# wakem 开发文档

本文档面向 wakem 开发者，包含架构设计、技术决策、扩展 API 和测试等内容。

## Changelog

```bash
git log v0.1.5..HEAD > gitlog.txt
git diff v0.1.5 HEAD -- "*.rs" "*.md" > gitdiff.txt
```

## Code coverage

```bash
rustup component add llvm-tools
cargo install cargo-llvm-cov

# 生成覆盖率报告
cargo llvm-cov
```

使用 `cargo llvm-cov` 生成覆盖率报告，找出需要提升测试覆盖率的代码路径。

```bash
pkill -f "wakem daemon" 2>/dev/null; sleep 1; pkill -9 -f "wakem daemon" 2>/dev/null; sleep 1; echo "已清理"
```

## 架构设计

### 技术决策

| 决策 | 理由 |
|------|------|
| **Rust** | 原生性能无 GC 暂停；编译时内存安全；windows-rs 提供完整 Win32 绑定；单文件分发 |
| **TOML 配置** | 比 JSON 更友好支持注释；有明确类型系统；toml crate 成熟稳定；Rust 社区广泛使用 |
| **客户端-服务端架构** | 权限分离（守护进程需管理员权限）；客户端崩溃不影响核心；支持远程控制（CLI 尚未实现）；各组件可独立测试 |
| **TCP 而非 Named Pipe** | 跨平台行为一致；端口区分实例更简单；天然支持网络通信；可用标准网络工具调试 |

### 平台抽象层

`src/platform/` 采用三层架构，将平台特异性代码与非平台特异性代码严格分离：

```
Layer 1: Types & traits    Layer 2: Common logic     Layer 3: Platform modules
┌───────────────────┐     ┌───────────────────┐     ┌───────────────────┐
│ traits.rs (19)    │     │ common/ (8 files)  │     │ windows/ (13)     │
│   窗口: ApiBase   │     │   window_manager   │     │   window_api      │
│     (_inner) +    │     │   input_device     │     │   window_manager  │
│     Ops/State/    │     │   output_helpers   │     │   output_device   │
│     Monitor/      │     │   key_names        │     │   tray/input      │
│     Foreground/   │     │   launcher         │     │   notification    │
│     Switching →   │     │   tray             │     │   context         │
│     WndMgrTrait   │     │   window_preset    │     │   platform_utils  │
│   WndMgrExt       │     │   app_control      │     │   event_hook      │
│   Input/Output    │     │                    │     │   window_preset   │
│   PresetMgr       │     │ 纯计算逻辑         │     │   app_control     │
│   Notif/Launcher  │     │ 无平台 API 调用    │     │ macos/ (19)       │
│   EventHook       │     │                    │     │   同构+native_api │
│   PlatformUtils   │     │                    │     │ mock/             │
│   Context/Tray    │     │                    │     │   mock_window_api │
│   AppCtrl/Factory │     │                    │     │                   │
│ types.rs/macros.rs│     │                    │     │                   │
└───────────────────┘     └───────────────────┘     └───────────────────┘
       ↑                        ↑                         ↑
  定义跨平台接口          提供默认实现和纯逻辑      平台特定的 API 薄封装
```

**核心模式**：`_inner` 委托模式（平台只实现原子操作）、组合 trait（`WindowManagerTrait` 组合 5 个基础 trait）、宏减少样板、`CommonWindowApi` 跨平台高级操作、`mock/` 测试辅助。

### 进程模型

```
wakem tray (主进程)
├── 主线程: 平台消息循环 (托盘图标 + 右键菜单)
├── Tokio 线程: IPC 客户端 + 心跳 + 命令处理
│   ├── 连接重试 (最多 10 次, 500ms 间隔, 同时监听 Exit)
│   ├── 命令分发 (ToggleActive / ReloadConfig / Exit ...)
│   └── 心跳 (1000ms, 3 次失败重连, 重连 3 次失败退出)
└── [可选] Daemon 线程: 自动启动守护进程

wakem daemon (守护进程)
├── Tokio 多线程运行时
│   ├── IPC 服务器 (TCP + HMAC-SHA256 + Zeroizing 密钥)
│   ├── 消息处理器 / 配置热重载
│   ├── 输入映射引擎 (批量 50 事件/100μs) + Hyper 键管理
│   └── 窗口事件处理 (自动应用预设)
├── Raw Input 线程 → 输入桥接 (sync→async mpsc)
└── 窗口事件钩子 → 窗口事件桥接 (sync→async mpsc)
```

**退出流程**：

- **Tray**：Exit → `Shutdown` IPC → `on_exit()` → `stop_tray()` (PostMessageW WM_CLOSE) → 删除托盘图标 → Join 线程 → 若自动启动 daemon 则 Join/force_kill
- **Daemon**：`ShutdownSignal` (watch::channel) 广播 → tokio 任务退出 → 设置 shutdown_flag → 桥接线程 join → 500ms 等待 → Join std 线程

---

## 功能概览

- **窗口管理**：居中/边缘/半屏、循环调整宽高、固定/原生比例、跨显示器移动、同进程切换、置顶/最小化/最大化/还原/关闭、绝对坐标移动/调整大小、调试信息/通知、窗口预设
- **键盘增强**：键位重映射（含修饰键组合 → Hyper）、Hyper 虚拟修饰键、快捷键层（Hold/Toggle）、导航层（Vim HJKL）、上下文感知快捷键、快速启动
- **鼠标增强**：按钮重映射（配置已定义，运行时待实现）、滚轮加速/反转/速度调节、水平滚动
- **系统功能**：系统托盘、配置重载/保存、自定义图标、多实例（`--instance N`）、实例发现（`wakem instances`）
- **高级功能**：窗口预设（保存/恢复/自动应用）、上下文感知快捷键、TCP + HMAC-SHA256 IPC、通配符匹配、宏录制回放（详见 [macros.md](macros.md)）

---

## IPC 协议

位置: `src/ipc/`

### 安全特性

| 特性 | 说明 |
|------|------|
| IP 白名单 | 仅允许本地连接（RFC 1918 + 环回），拒绝 IPv6 |
| HMAC-SHA256 | Challenge-response（5s 超时），`Zeroizing<String>` 密钥，常量时间比较 |
| 连接限流 | 60s 内每 IP 最多 5 次，最多追踪 1000 IP |
| 空闲超时 | 30s（一次性命令） |
| 协议版本 | `IPC_PROTOCOL_VERSION = 1`，认证成功后协商 |
| 消息大小 | `IPC_MAX_MESSAGE_SIZE = 1MB` |

### 消息列表

| 方向 | 消息 | 状态 | 说明 |
|------|------|------|------|
| C→S | `SetConfig` / `ReloadConfig` / `SaveConfig` | ✅ | 配置管理 |
| C→S | `GetStatus` / `SetActive` | ✅ | 状态查询与控制 |
| C→S | `StartMacroRecording` / `StopMacroRecording` / `PlayMacro` / `GetMacros` / `DeleteMacro` / `BindMacro` | ✅ | 宏操作 |
| C→S | `Shutdown` | ✅ | 关闭守护进程 |
| C→S | `GetNextKeyInfo` | Stub | 返回 "not implemented" 错误 |
| S→C | `StatusResponse` / `ConfigLoaded` / `ConfigError` / `Error` / `MacroRecordingResult` / `MacrosList` / `Success` | ✅ | 响应消息 |
| S→C | `NextKeyInfo` | 预留 | 未使用 |
| 双向 | `Ping/Pong` | ✅ | 心跳检测 |

### 实例发现

位置: `src/ipc/discovery.rs`

- `discover_instances()` 并行扫描所有可能的实例端口（0-255），100ms 超时
- 端口方案：`IPC_BASE_PORT(57427) + instance_id`
- 返回 `Vec<InstanceInfo>`，包含 `id`、`address`、`active` 字段
- CLI 命令：`wakem instances` 列出所有发现的实例
- 全局 `-i` / `--instance` 参数（默认 0）用于指定目标实例

---

## 预留扩展 API

以下 API 和功能已在代码中定义但部分尚未完全使用，为未来扩展预留。

### 触发器类型 (`Trigger`)

位置: `src/types/mapping.rs`

| 触发器类型 | 状态 | 说明 |
|-----------|------|------|
| `Key { ... }` | ✅ | 键盘按键触发（支持扫描码/虚拟键码/修饰键） |
| `MouseButton { ... }` | ✅ | 鼠标按钮触发（支持修饰键组合，`matches()` 已实现） |
| `HotString { trigger }` | 预留 | 热字符串（文本扩展），类似 AutoHotkey 的 ::btw::be right back:: |
| `Chord(Vec<Trigger>)` | 预留 | 组合触发（多个按键按顺序），如 `Ctrl,K,C` |
| `Timer { interval_ms }` | 预留 | 定时触发器，用于定时执行任务 |
| `Always` | 预留 | 总是触发的规则 |

### 映射规则 API

位置: `src/types/mapping.rs`

`MappingRule::new().with_name().with_context().matches()` — 创建、命名、添加上下文、匹配事件。

上下文条件 (`ContextCondition`) 支持：`process_name` / `window_class` / `window_title` / `executable_path`（均支持通配符）

### 层管理 API

位置: `src/types/layer.rs`, `src/runtime/layer_manager.rs`

- `Layer`: 层定义（名称、激活键、模式、映射规则）
- `LayerStack`: 管理层激活/停用/Hold/Toggle
- `LayerManager`: 处理输入事件的层分发

层模式：`Hold`（按住激活，释放退出）、`Toggle`（按一次进入，再按退出）

### 通配符匹配

位置: `src/types/context.rs` → `wildcard_match()`

`*` 匹配任意字符序列（连续 `*` 合并优化），`?` 匹配单个字符，大小写不敏感，动态规划 O(m*n)。

### 配置验证规则

位置: `src/config.rs` → `Config::validate()`

- 日志级别（trace/debug/info/warn/warning/error）
- 端口 ≥ 1024，实例 ID 0-255
- 宏绑定引用存在性，层激活键非空，空宏步骤警告
- 滚轮加速度 0.1-10.0，滚轮速度 > 0
- icon_path 存在性（可 `WAKEM_SKIP_ICON_VALIDATION` 跳过）
- launch 命令非空
- keyboard.remap：源键有效，目标为有效键名/窗口动作/修饰键组合
- window.shortcuts：快捷键格式和窗口动作格式合法

### 待实现扩展

- **鼠标按钮重映射** — 完成 `MouseConfig.button_remap`（配置已定义，底层 `MouseButton`/`Trigger::MouseButton`/`send_mouse_button()` 已就绪，但 `validate()`/`get_all_rules()`/运行时逻辑未实现）
- **组合触发 (Chord)** — 实现顺序按键触发
- **热字符串 (HotString)** — 实现文本扩展功能
- **跨平台抽象层完善** — 为 macOS/Linux 移植做准备
- **宏回放中的 Window/Launch 动作** — 当前 MacroPlayer 跳过 Window 和 Launch 动作，仅输出警告日志

---

## 测试

### 单元测试

```bash
# 运行所有单元测试
cargo test

# 按前缀筛选
cargo test ut_           # 单元测试
cargo test it_           # 集成测试
cargo test prop_         # 属性测试
cargo test platform_     # 平台特定测试
```

测试文件在 `tests/` 目录下，命名规范: `ut_<模块>.rs` / `it_<描述>.rs` / `prop_<描述>.rs` / `platform_<平台>_<描述>.rs`。

### E2E 测试

E2E 测试需要真实的桌面环境，默认 `#[ignore]`，不会影响常规 `cargo test`。

```powershell
# 窗口管理
cargo test --test e2e_windows_window -- --ignored --test-threads=1

# 程序启动器
cargo test --test e2e_windows_launcher -- --ignored --test-threads=1

# 托盘退出行为
cargo test --test e2e_windows_tray_exit -- --ignored --test-threads=1

# macOS 窗口管理
cargo test --test e2e_macos -- --ignored --test-threads=1

# 托盘退出 (PowerShell 辅助脚本，更完整)
scripts/e2e_tray_exit.ps1
scripts/e2e_tray_exit.ps1 -TestNoDaemon
scripts/e2e_tray_exit.ps1 -TestWithDaemon
```

### 性能基准

使用 Criterion 框架，运行命令：`cargo bench`

#### 基准测试结果

| Benchmark | 平均时间 | 说明 |
|-----------|---------|------|
| `window_center_calculation` | ~270 ps | 窗口居中计算 |
| `trigger_key_match` / `mapping_rule_match` | ~2.0 ns | 触发器/规则匹配 |
| `action_creation` | ~14.3 ns | Action 枚举创建 |
| `json_deserialization` / `json_serialization` | ~65ns / ~205ns | JSON 序列化 |
| `context_match` | ~120 ns | 上下文条件匹配 |
| `layer_stack_operations` / `real_world_layer_operations` | ~11.7μs / ~1.32μs | 层操作 |

**核心路径**：触发器/规则匹配 ~2ns，窗口计算 270ps，序列化 ~205ns/~65ns，层操作 ~1.17μs/次。

#### 基准测试文件

```
benches/
├── basic_benchmarks.rs    # 跨平台基准测试（9 个 benchmark）
└── macos/
    └── macos_bench.rs     # macOS 专用基准（5 个 benchmark）[macOS only]
```
