# wakem 开发文档

本文档面向 wakem 开发者，包含架构设计、技术决策、扩展 API 和测试等内容。

## Changelog

```bash
git log v0.1.2..HEAD > gitlog.txt
git diff v0.1.2 HEAD -- "*.rs" "*.md" > gitdiff.txt
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

## 参考项目分析

### AutoHotkey 架构分析

**AutoHotkey** 是 Windows 平台上最著名的自动化脚本语言和热键工具。

#### 核心组件

1. **脚本引擎** - 完整的解释型脚本语言，支持变量、函数、对象、类
2. **钩子系统** - 低级键盘/鼠标钩子实现全局热键
3. **热键管理** - 热键变体（不同条件下触发不同动作）
4. **窗口管理** - 窗口查找、操作、控制
5. **消息循环** - 使用 `MsgSleep` 替代 `Sleep` 确保钩子事件及时处理

#### 可借鉴的设计

1. **消息驱动架构** - 基于 Windows 消息循环，避免使用 `Sleep`
2. **热键优先级和变体** - 灵活的优先级系统和条件触发
3. **窗口搜索机制** - 多条件窗口搜索（标题、类名、PID、进程名）
4. **线程中断控制** - `Critical` 和 `AllowInterruption` 机制
5. **钩子状态管理** - 动态安装/卸载钩子，减少系统开销

## 架构设计

### 技术决策

| 决策 | 理由 |
|------|------|
| **Rust** | 原生性能无 GC 暂停；编译时内存安全；windows-rs 提供完整 Win32 绑定；单文件分发 |
| **TOML 配置** | 比 JSON 更友好支持注释；有明确类型系统；toml crate 成熟稳定；Rust 社区广泛使用 |
| **客户端-服务端架构** | 权限分离（守护进程需管理员权限）；客户端崩溃不影响核心；支持远程控制；各组件可独立测试 |
| **TCP 而非 Named Pipe** | 跨平台行为一致；端口区分实例更简单；天然支持网络通信；可用标准网络工具调试 |

### 平台抽象层

`src/platform/` 采用三层架构，将平台特异性代码与非平台特异性代码严格分离：

```
Layer 1: Types & traits          Layer 2: Common logic           Layer 3: Platform modules
┌─────────────────────────┐     ┌─────────────────────────┐     ┌─────────────────────────┐
│ traits.rs               │     │ common/                 │     │ windows/                │
│   WindowApiBase (_inner)│     │   window_manager.rs     │     │   window_api.rs         │
│   WindowManagerExt      │     │   input_device.rs       │     │   window_manager.rs     │
│   OutputDeviceTrait     │     │   output_helpers.rs     │     │   output_device.rs      │
│   NotificationService   │     │   launcher.rs           │     │   tray.rs               │
│   ...                   │     │   tray.rs               │     │   ...                   │
│ types.rs                │     │   window_preset.rs      │     │ macos/                  │
│   MonitorInfo           │     │                         │     │   window_api.rs         │
│   WindowContext         │     │ 纯计算逻辑，无平台 API  │     │   ...                   │
│   NotificationInitCtx   │     │ 调用                    │     │                         │
│ macros.rs               │     │                         │     │ 仅保留平台 API 薄封装   │
│   impl_*! 宏减少样板    │     │                         │     │ 使用宏减少重复代码      │
└─────────────────────────┘     └─────────────────────────┘     └─────────────────────────┘
         ↑                              ↑                               ↑
    定义跨平台接口              提供默认实现和纯逻辑           平台特定的 API 调用
```

**核心模式**：

- **`_inner` 模式**：`WindowApiBase` 的公共方法有默认实现，委托到 `_inner` 抽象方法。平台只需实现 `_inner` 方法（最小原子操作），公共逻辑由 trait 默认方法提供。
- **宏减少样板**：`impl_platform_factory_methods!`、`impl_context_provider!`、`impl_tray_lifecycle!` 等宏让平台模块只保留平台特异性逻辑。
- **`CommonWindowApi`**：`common/window_manager.rs` 提供高级窗口操作（居中、半屏、循环比例等）的跨平台实现，使用 `find_next_ratio` 修复浮点截断 bug。

### 进程模型

```
wakem tray (主进程)
├── 主线程: Windows 消息循环 (托盘图标 + 右键菜单)
├── Tokio 线程: IPC 客户端 + 命令处理
│   ├── 连接重试 (tokio::select! 同时监听 Exit 命令)
│   └── 命令分发 (ToggleActive / ReloadConfig / Exit ...)
└── [可选] Daemon 线程: 自动启动守护进程

wakem daemon (守护进程)
├── Tokio 异步运行时
│   ├── IPC 服务器 (TCP + HMAC-SHA256 认证 + Zeroizing 密钥)
│   ├── 配置热重载
│   └── 输入映射引擎
└── 平台钩子线程 (Raw Input / CGEvent)
```

**退出流程**：

```
用户点击 "Exit" → callback(AppCommand::Exit)
  → cmd_tx.blocking_send(Exit)
  → tokio 线程: on_exit() → stop_tray()
    → PostMessageW(WM_CLOSE) [跨线程, 使用 OnceLock 存储 HWND]
  → 主线程: WM_CLOSE → WM_DESTROY
    → Shell_NotifyIconW(NIM_DELETE) [删除托盘图标]
    → PostQuitMessage(0) [终止消息循环]
  → 进程退出
```

---

## 功能概览

### 窗口管理
- 窗口居中 / 移动到边缘 / 半屏显示
- 循环调整宽度/高度（多种比例）
- 固定比例窗口 / 原生比例窗口
- 跨显示器移动 / 同进程窗口切换 (Alt+`)
- 窗口置顶 / 最小化/最大化/还原/关闭
- 窗口绝对坐标移动 / 大小调整
- 显示调试信息 / 显示通知
- 保存/加载/应用窗口预设

### 键盘增强
- 键位重映射（包括修饰键组合映射：CapsLock → Hyper）
- 快捷键层系统（Hold/Toggle 模式）
- 导航层（Vim 风格 HJKL）
- 应用快捷键（上下文感知）
- 快速启动（支持带参数命令）

### 鼠标增强
- 鼠标事件捕获
- 滚轮加速 / 反转 / 速度调节
- 水平滚动（Shift + 滚轮）
- 音量控制（RightAlt + 滚轮）
- 亮度控制（RightCtrl + 滚轮）

### 系统功能
- 系统托盘（带通知功能）
- 配置命令行重载 (`wakem reload`)
- 配置保存到文件 (`wakem save`)
- 自定义托盘图标
- 多实例支持（`--instance N` 参数）
- 实例发现和管理 (`wakem instances`)

### 高级功能
- 窗口预设（保存/恢复/自动应用布局）
- 上下文感知快捷键（进程名/标题/类/路径匹配）
- 网络通信（TCP + 远程控制 + HMAC-SHA256 认证）
- 通配符匹配（支持 `*` 和 `?`，大小写不敏感）
- 宏录制回放系统（详见 [macros.md](macros.md)）

---

## IPC 协议

位置: `src/ipc/`

### 安全特性

| 特性 | 说明 |
|------|------|
| IP 白名单 | 仅允许本地连接 |
| HMAC-SHA256 认证 | Challenge-response 机制，密钥用 `Zeroizing<String>` 保护 |
| 连接限流 | 防止暴力破解 |
| 双空闲超时 | SHORT(30s) 用于一次性命令，LONG(600s) 用于 tray 长连接 |
| 协议版本 | `IPC_PROTOCOL_VERSION = 1`，连接时协商 |

### 消息列表

| 方向 | 消息 | 状态 | 说明 |
|------|------|------|------|
| C→S | `SetConfig` | ✅ | 发送配置到服务端 |
| C→S | `ReloadConfig` | ✅ | 重新加载配置 |
| C→S | `SaveConfig` | ✅ | 保存配置到文件 |
| C→S | `GetStatus` | ✅ | 获取当前状态 |
| C→S | `SetActive` | ✅ | 启用/禁用映射 |
| C→S | `GetNextKeyInfo` | 预留 | 获取下一个按键信息（调试用） |
| C→S | `StartMacroRecording` | ✅ | 开始录制宏 |
| C→S | `StopMacroRecording` | ✅ | 停止录制宏 |
| C→S | `PlayMacro` | ✅ | 播放宏 |
| C→S | `GetMacros` | ✅ | 获取宏列表 |
| C→S | `DeleteMacro` | ✅ | 删除宏 |
| C→S | `BindMacro` | ✅ | 绑定宏到触发键 |
| C→S | `RegisterMessageWindow` | ✅ | 注册消息窗口句柄 |
| C→S | `Shutdown` | ✅ | 关闭守护进程 |
| S→C | `StatusResponse` | ✅ | 状态响应 |
| S→C | `ConfigLoaded` | ✅ | 配置已加载 |
| S→C | `ConfigError` | ✅ | 配置加载错误 |
| S→C | `NextKeyInfo` | 预留 | 下一个按键信息 |
| S→C | `Error` | ✅ | 错误响应 |
| S→C | `MacroRecordingResult` | ✅ | 宏录制结果 |
| S→C | `MacrosList` | ✅ | 宏列表响应 |
| S→C | `Success` | ✅ | 成功响应 |
| 双向 | `Ping/Pong` | ✅ | 心跳检测 |

---

## 预留扩展 API

以下 API 和功能已在代码中定义但部分尚未完全使用，为未来扩展预留。

### 触发器类型 (`Trigger`)

位置: `src/types/mapping.rs`

| 触发器类型 | 状态 | 说明 |
|-----------|------|------|
| `Key { ... }` | ✅ | 键盘按键触发（支持扫描码/虚拟键码/修饰键） |
| `MouseButton { ... }` | 已定义 | 鼠标按钮触发（可用于未来鼠标映射） |
| `HotString { trigger }` | 预留 | 热字符串（文本扩展），类似 AutoHotkey 的 ::btw::be right back:: |
| `Chord(Vec<Trigger>)` | 预留 | 组合触发（多个按键按顺序），如 `Ctrl,K,C` |
| `Timer { interval_ms }` | 预留 | 定时触发器，用于定时执行任务 |
| `Always` | 预留 | 总是触发的规则 |

### 映射规则 API

位置: `src/types/mapping.rs`

| 方法 | 说明 |
|------|------|
| `MappingRule::new()` | 创建新规则 |
| `with_name()` | 设置规则名称 |
| `with_context()` | 添加上下文条件 |
| `matches()` | 检查事件是否匹配规则 |

上下文条件 (`ContextCondition`) 支持：
- `process_name`: 进程名匹配（通配符）
- `window_class`: 窗口类名匹配（通配符）
- `window_title`: 窗口标题匹配（通配符）
- `executable_path`: 可执行路径匹配（通配符）

### 层管理 API

位置: `src/types/layer.rs`, `src/runtime/layer_manager.rs`

- `Layer`: 层定义（名称、激活键、模式、映射规则）
- `LayerStack`: 管理层激活/停用/Hold/Toggle
- `LayerManager`: 处理输入事件的层分发

层模式：`Hold`（按住激活，释放退出）、`Toggle`（按一次进入，再按退出）

### 通配符匹配

位置: `src/config.rs` → `wildcard_match()` 和 `WindowPreset::wildcard_match()`

- `*` 匹配任意字符序列（连续 `*` 会被合并优化）
- `?` 匹配单个字符
- 大小写不敏感匹配
- 动态规划算法，时间复杂度 O(m*n)

### 配置验证规则

位置: `src/config.rs` → `Config::validate()`

- 日志级别有效性（trace/debug/info/warn/error）
- 端口范围（1024-65535）
- 实例 ID 范围（0-255）
- 宏绑定引用的宏存在性
- 层激活键非空
- 空宏步骤警告
- 鼠标滚轮加速度范围（0.1-10.0）
- 鼠标滚轮速度正数

### 待实现扩展

- **鼠标按钮重映射** — 完成 `MouseConfig.button_remap` 功能
- **组合触发 (Chord)** — 实现顺序按键触发
- **热字符串 (HotString)** — 实现文本扩展功能
- **跨平台抽象层完善** — 为 macOS/Linux 移植做准备

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

# 托盘退出 (PowerShell 辅助脚本，更完整)
scripts/e2e_tray_exit.ps1
scripts/e2e_tray_exit.ps1 -TestNoDaemon
scripts/e2e_tray_exit.ps1 -TestWithDaemon
```

#### 窗口管理测试

> `tests/e2e_windows_window.rs` | 仅 Windows

| 测试名 | 说明 |
|--------|------|
| `test_get_foreground_window_info` | 获取前台窗口信息（标题、位置、大小、显示器工作区） |
| `test_get_window_info_by_handle` | 通过句柄获取窗口信息 |
| `test_get_debug_info` | 获取调试信息字符串 |
| `test_move_to_center` | 窗口居中 |
| `test_move_to_edge` | 窗口移动到边缘 |
| `test_set_half_screen` | 半屏显示（左/右） |
| `test_loop_width_cycle` | 循环调整宽度（多种预设比例） |
| `test_loop_height_cycle` | 循环调整高度（多种预设比例） |
| `test_set_fixed_ratio_16_9_and_4_3` | 固定比例窗口（16:9、4:3 等） |
| `test_set_window_frame` | 绝对坐标移动和调整大小 |
| `test_minimize_and_restore_window` | 最小化后还原 |
| `test_maximize_and_restore_window` | 最大化后还原 |
| `test_toggle_topmost` | 置顶/取消置顶切换 |
| `test_close_window` | 关闭窗口 |
| `test_switch_to_next_window_of_same_process` | 切换到同进程下一个窗口 |
| `test_switch_cycles_through_three_windows` | 3 个窗口循环切换验证 |
| `test_switch_cycles_through_four_windows` | 4 个窗口循环切换验证 |
| `test_single_window_does_not_panic` | 单窗口时切换不报错 |
| `test_get_app_visible_windows` | 获取应用可见窗口 |
| `test_get_app_visible_windows_finds_notepad` | 窗口枚举能找到 Notepad |
| `test_explorer_multi_process_window_enumeration` | Explorer 多进程窗口枚举 |

#### 程序启动器测试

> `tests/e2e_windows_launcher.rs` | 仅 Windows

| 测试名 | 说明 |
|--------|------|
| `test_launch_simple_program` | 启动计算器 (calc.exe) |
| `test_launch_program_with_args` | 启动记事本并打开指定文件 |
| `test_launcher_parse_command_and_launch` | `parse_command` -> `launch` 完整流程 |
| `test_launch_program_with_multiple_args` | 多参数启动 (ping 命令) |
| `test_launch_nonexistent_program` | 启动不存在的程序应返回错误 |
| `test_launch_system_program_cmd` | 启动 cmd.exe 并执行命令 |

#### 托盘退出行为测试

> `tests/ut_tray_exit.rs` + `tests/e2e_windows_tray_exit.rs` | 仅 Windows

验证托盘进程在不同场景下能否正常退出（包括图标消失）。

**单元测试**（`cargo test --test ut_tray_exit`，自动运行）：

| 测试名 | 说明 |
|--------|------|
| `test_exit_during_connection_phase` | 连接阶段收到 Exit 命令立即退出 |
| `test_exit_during_connection_phase_with_delay` | 重试中途收到 Exit 命令也能退出 |
| `test_other_commands_ignored_during_connection` | 连接阶段其他命令不崩溃 |
| `test_channel_close_exits_handler` | channel 关闭时正常退出（不调 on_exit） |
| `test_stop_tray_callable_from_any_thread` | `stop_tray()` 跨线程可用（OnceLock 修复） |
| `test_stop_tray_callable_without_init` | 未初始化时 `stop_tray()` 不 panic |

**E2E 测试**（需要桌面会话，默认 `#[ignore]`）：

| 测试名 | 说明 |
|--------|------|
| `test_tray_exit_without_daemon` | 无守护进程时 tray 退出 |
| `test_tray_exit_with_daemon_via_ipc` | IPC shutdown 触发完整退出流程 |
| `test_tray_restart_cycle` | 3 次重启不残留僵尸进程 |

#### 待实现的 E2E 测试

- `set_native_ratio()` — 原生比例窗口
- `move_to_monitor(Index)` — 按索引移动显示器
- `move_to_monitor_next()` / `move_to_monitor_prev()` — 跨显示器移动

#### E2E 测试设计要点

- 使用 `WindowManager` 调用真实 Windows API
- 通过 `Command::new("notepad.exe").spawn()` 启动真实进程
- `wait_for_window` 辅助函数轮询等待窗口出现（最长 5 秒超时）
- 每个测试结束后自动 `taskkill /IM notepad.exe /F` 清理
- `#[cfg(target_os = "windows")]` 条件编译，非 Windows 平台提供空占位测试

### 性能基准

使用 Criterion 框架，运行命令：`cargo bench`

#### 基准测试结果

| Benchmark | 平均时间 | 说明 |
|-----------|---------|------|
| `window_center_calculation` | ~270 ps | 窗口居中计算（纯数学运算） |
| `trigger_key_match` | ~2.0 ns | 触发器按键匹配 |
| `mapping_rule_match` | ~2.0 ns | 映射规则匹配（含上下文） |
| `action_creation` | ~14.3 ns | Action 枚举创建 |
| `json_deserialization` | ~65.0 ns | JSON 反序列化 |
| `context_match` | ~120 ns | 上下文条件匹配（进程名+窗口类） |
| `json_serialization` | ~205 ns | JSON 序列化 |
| `layer_stack_operations` | ~11.7 μs | 层栈激活/停用操作（10 次循环） |
| `real_world_layer_operations` | ~1.32 μs | 真实场景层操作（10 次迭代） |

**核心路径**：触发器匹配和规则匹配均在纳秒级 (~2ns)，窗口计算亚纳秒级 (270ps)。
**序列化**：序列化 ~205ns / 反序列化 ~65ns，远低于网络延迟。
**层管理**：10 次操作仅需 ~11.7μs（平均 1.17μs/次）。

#### 基准测试文件

```
benches/
├── basic_benchmarks.rs    # 跨平台基准测试（8 个 benchmark）
└── macos/
    └── macos_bench.rs     # macOS 专用基准 [macOS only]
```
