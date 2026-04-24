# wakem 开发文档

本文档面向 wakem 开发者，包含架构设计、技术决策、扩展 API 和性能数据等内容。

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

---

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

---

## 技术决策记录

### 为什么选择 Rust?

1. **性能** - 原生性能，无 GC 暂停
2. **安全性** - 编译时内存安全保证
3. **Windows 支持** - windows-rs crate 提供完整的 Windows API 绑定
4. **单文件分发** - 可以编译为单个可执行文件

### 为什么选择 TOML 配置?

1. **可读性** - 比 JSON 更友好，支持注释
2. **类型安全** - 有明确的类型系统
3. **Rust 生态** - toml crate 成熟稳定
4. **用户熟悉** - Rust 社区广泛使用

### 为什么采用客户端-服务端架构?

1. **权限分离** - 守护进程需要管理员权限，客户端不需要
2. **稳定性** - 客户端崩溃不影响核心功能
3. **灵活性** - 支持远程控制和多个客户端
4. **可测试性** - 各组件可以独立测试

### 为什么使用 TCP 而非 Named Pipe?

1. **跨平台兼容** - TCP 在所有平台上行为一致
2. **多实例支持** - 通过端口区分实例更简单
3. **远程控制** - 天然支持网络通信
4. **调试便利** - 可以使用标准网络工具调试

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

## 预留扩展 API

以下 API 和功能已在代码中定义但部分尚未完全使用，为未来扩展预留：

### 1. 触发器类型 (`Trigger`)

位置: `src/types/mapping.rs`

| 触发器类型 | 状态 | 说明 |
|-----------|------|------|
| `Key { ... }` | 已使用 | 键盘按键触发（支持扫描码/虚拟键码/修饰键） |
| `MouseButton { ... }` | 已定义 | 鼠标按钮触发（可用于未来鼠标映射） |
| `HotString { trigger }` | 预留 | 热字符串（文本扩展），类似 AutoHotkey 的 ::btw::be right back:: |
| `Chord(Vec<Trigger>)` | 预留 | 组合触发（多个按键按顺序），如 `Ctrl,K,C` |
| `Timer { interval_ms }` | 预留 | 定时触发器，用于定时执行任务 |
| `Always` | 预留 | 总是触发的规则 |

### 2. 鼠标按钮重映射

位置: `src/config.rs` → `MouseConfig`

`button_remap` 字段已定义但功能待实现。可用于将鼠标侧键映射为其他功能。

### 3. 配置验证规则

位置: `src/config.rs` → `Config::validate()`

当前实现的验证规则：
- 日志级别有效性检查（trace/debug/info/warn/error）
- 端口范围检查（1024-65535）
- 实例 ID 范围检查（0-255）
- 宏绑定引用的宏存在性检查
- 层激活键非空检查
- 空宏步骤警告
- 鼠标滚轮加速度范围检查（0.1-10.0）
- 鼠标滚轮速度正数检查

### 4. IPC 消息协议完整列表

位置: `src/ipc/mod.rs` → `Message` 枚举

| 消息方向 | 消息 | 状态 | 说明 |
|---------|------|------|------|
| C→S | `SetConfig` | 已使用 | 发送配置到服务端 |
| C→S | `ReloadConfig` | 已使用 | 重新加载配置 |
| C→S | `SaveConfig` | 已使用 | 保存配置到文件 |
| C→S | `GetStatus` | 已使用 | 获取当前状态 |
| C→S | `SetActive` | 已使用 | 启用/禁用映射 |
| C→S | `GetNextKeyInfo` | 预留 | 获取下一个按键信息（用于调试） |
| C→S | `StartMacroRecording` | 已使用 | 开始录制宏 |
| C→S | `StopMacroRecording` | 已使用 | 停止录制宏 |
| C→S | `PlayMacro` | 已使用 | 播放宏 |
| C→S | `GetMacros` | 已使用 | 获取宏列表 |
| C→S | `DeleteMacro` | 已使用 | 删除宏 |
| C→S | `BindMacro` | 已使用 | 绑定宏到触发键 |
| C→S | `RegisterMessageWindow` | 已使用 | 注册消息窗口句柄 |
| C→S | `Shutdown` | 已使用 | 关闭守护进程 |
| S→C | `StatusResponse` | 已使用 | 状态响应 |
| S→C | `ConfigLoaded` | 已使用 | 配置已加载 |
| S→C | `ConfigError` | 已使用 | 配置加载错误 |
| S→C | `NextKeyInfo` | 预留 | 下一个按键信息 |
| S→C | `Error` | 已使用 | 错误响应 |
| S→C | `MacroRecordingResult` | 已使用 | 宏录制结果 |
| S→C | `MacrosList` | 已使用 | 宏列表响应 |
| S→C | `Success` | 已使用 | 成功响应 |
| 双向 | `Ping/Pong` | 已使用 | 心跳检测 |

### 5. 层管理 API

位置: `src/types/layer.rs`, `src/runtime/layer_manager.rs`

核心 API：
- `Layer`: 层定义（名称、激活键、模式、映射规则）
- `LayerStack`: 管理层激活/停用/Hold/Toggle
- `LayerManager`: 处理输入事件的层分发

层模式支持：
- `Hold`: 按住激活，释放退出
- `Toggle`: 按一次进入，再按一次退出

### 6. 映射规则 API

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

### 7. 通配符匹配实现细节

位置: `src/config.rs` → `wildcard_match()` 和 `WindowPreset::wildcard_match()`

通配符匹配已完整实现：
- `*` 匹配任意字符序列（连续 `*` 会被合并优化）
- `?` 匹配单个字符
- 大小写不敏感匹配
- 动态规划算法确保正确性，时间复杂度 O(m*n)

---

## 扩展建议

- **鼠标按钮重映射** - 完成 `MouseConfig.button_remap` 功能
- **组合触发 (Chord)** - 实现顺序按键触发
- **热字符串 (HotString)** - 实现文本扩展功能
- **跨平台抽象层完善** - 为 macOS/Linux 移植做准备

---

## 性能基准测试

使用 Criterion 框架进行性能测试，运行命令：`cargo bench`

### 测试环境

- OS: Windows 10/11, macOS
- CPU: x86_64, ARM64 (Apple Silicon)
- 编译: release + debuginfo (opt-level = 3)

### 基准测试结果

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

### 性能分析

**核心路径性能优秀**:
- 触发器匹配和规则匹配均在 **纳秒级** (~2ns)
- 单次输入事件处理延迟极低，满足实时性要求
- 窗口计算为 **亚纳秒级** (270ps)，几乎无开销

**序列化开销可接受**:
- JSON 序列化/反序列化用于 IPC 通信和配置持久化
- 序列化 ~205ns / 反序列化 ~65ns，远低于网络延迟

**层管理效率高**:
- 10 次层操作仅需 ~11.7μs（平均 1.17μs/次）
- 支持复杂的 Hold/Toggle 场景切换

### 基准测试文件

```
benches/
├── basic_benchmarks.rs    # 跨平台基准测试（8 个 benchmark）
└── macos/
    └── macos_bench.rs     # macOS 专用基准 [macOS only]
```

## 真实集成测试

> 位置: `tests/e2e_windows_window.rs` | 仅 Windows 平台

与 `tests/` 目录下的其他 mock 测试不同，真实集成测试会在**桌面启动真实窗口**并验证实际行为。

所有测试默认 `#[ignore]`，不会影响常规 `cargo test`。

### 运行方式

```bash
# 运行窗口管理真实集成测试
cargo test --test e2e_windows_window -- --ignored --test-threads=1

# 运行程序启动器真实集成测试
cargo test --test e2e_windows_launcher -- --ignored --test-threads=1

# 单个测试
cargo test --test e2e_windows_window test_get_foreground_window_info -- --ignored --test-threads=1
cargo test --test e2e_windows_launcher test_launch_simple_program -- --ignored --test-threads=1
```

### 测试用例列表

#### 窗口信息获取

| 测试名 | 说明 |
|--------|------|
| `test_get_foreground_window_info` | 获取前台窗口信息（标题、位置、大小、显示器工作区） |
| `test_get_window_info_by_handle` | 通过句柄获取窗口信息 |
| `test_get_debug_info` | 获取调试信息字符串 |

#### 窗口位置与大小

| 测试名 | 说明 |
|--------|------|
| `test_move_to_center` | 窗口居中 |
| `test_move_to_edge` | 窗口移动到边缘 |
| `test_set_half_screen` | 半屏显示（左/右） |
| `test_loop_width_cycle` | 循环调整宽度（多种预设比例） |
| `test_loop_height_cycle` | 循环调整高度（多种预设比例） |
| `test_set_fixed_ratio_16_9_and_4_3` | 固定比例窗口（16:9、4:3 等） |
| `test_set_window_frame` | 绝对坐标移动和调整大小 |

#### 窗口状态控制

| 测试名 | 说明 |
|--------|------|
| `test_minimize_and_restore_window` | 最小化后还原 |
| `test_maximize_and_restore_window` | 最大化后还原 |
| `test_toggle_topmost` | 置顶/取消置顶切换 |
| `test_close_window` | 关闭窗口 |

#### 同进程窗口切换（Alt+`）

| 测试名 | 说明 |
|--------|------|
| `test_switch_to_next_window_of_same_process` | 切换到同进程下一个窗口 |
| `test_switch_cycles_through_three_windows` | 3 个窗口循环切换验证 |
| `test_switch_cycles_through_four_windows` | 4 个窗口循环切换验证 |
| `test_single_window_does_not_panic` | 单窗口时切换不报错 |

#### 窗口枚举

| 测试名 | 说明 |
|--------|------|
| `test_get_app_visible_windows` | 获取应用可见窗口 |
| `test_get_app_visible_windows_finds_notepad` | 窗口枚举能找到 Notepad |
| `test_explorer_multi_process_window_enumeration` | Explorer 多进程窗口枚举（不含系统窗口） |

#### 程序启动器 (Launcher)

> 位置: `tests/e2e_windows_launcher.rs` | 仅 Windows 平台

| 测试名 | 说明 |
|--------|------|
| `test_launch_simple_program` | 启动计算器 (calc.exe) |
| `test_launch_program_with_args` | 启动记事本并打开指定文件 |
| `test_launcher_parse_command_and_launch` | `parse_command` -> `launch` 完整流程 |
| `test_launch_program_with_multiple_args` | 多参数启动 (ping 命令) |
| `test_launch_nonexistent_program` | 启动不存在的程序应返回错误 |
| `test_launch_system_program_cmd` | 启动 cmd.exe 并执行命令 |

### 待实现测试

以下 API 尚未覆盖，欢迎补充：

- `set_native_ratio()` - 原生比例窗口
- `move_to_monitor(Index)` - 按索引移动显示器
- `move_to_monitor_next()` / `move_to_monitor_prev()` - 跨显示器移动

### 设计要点

- 使用 `WindowManager` 调用真实 Windows API
- 通过 `Command::new("notepad.exe").spawn()` 启动真实进程
- `wait_for_window` 辅助函数轮询等待窗口出现（最长 5 秒超时）
- 每个测试结束后自动 `taskkill /IM notepad.exe /F` 清理
- `#[cfg(target_os = "windows")]` 条件编译，非 Windows 平台提供空占位测试

