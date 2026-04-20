# wakem 开发文档

本文档包含 wakem 的开发记录、架构说明和开发计划。

## 目录结构

```
wakem/
├── Cargo.toml              # 项目配置
├── src/
│   ├── main.rs             # 统一入口（wakem）
│   ├── lib.rs              # 库导出
│   ├── cli.rs              # 命令行定义
│   ├── client.rs           # 客户端逻辑
│   ├── daemon.rs           # 守护进程逻辑
│   ├── config.rs           # 配置解析
│   ├── window.rs           # 消息窗口
│   ├── types/              # 类型定义
│   │   ├── mod.rs
│   │   ├── action.rs       # 动作定义（Key/Mouse/Window/Launch/System）
│   │   ├── input.rs        # 输入事件定义
│   │   ├── layer.rs        # 层管理
│   │   ├── macros.rs       # 宏录制和管理
│   │   └── mapping.rs      # 映射规则
│   ├── ipc/                # IPC 通信（统一使用 TCP）
│   │   ├── mod.rs          # 模块导出和消息协议
│   │   ├── client.rs       # TCP 客户端
│   │   ├── server.rs       # TCP 服务端
│   │   ├── auth.rs         # 挑战-响应认证
│   │   ├── security.rs     # IP 安全检查
│   │   └── discovery.rs    # 实例发现
│   ├── platform/           # 平台相关
│   │   └── windows/        # Windows 实现
│   │       ├── mod.rs
│   │       ├── input.rs
│   │       ├── hook.rs
│   │       ├── window_manager.rs
│   │       ├── window_preset.rs    # 窗口预设管理
│   │       ├── window_event_hook.rs # 窗口事件监听
│   │       ├── context.rs
│   │       ├── launcher.rs
│   │       ├── output.rs
│   │       └── tray.rs
│   └── runtime/            # 运行时逻辑
│       ├── mod.rs
│       ├── layer_manager.rs
│       ├── mapper.rs
│       └── macro_player.rs # 宏回放
├── tests/                  # 集成测试
│   ├── action_test.rs
│   ├── benchmark_test.rs
│   ├── config_parser_test.rs
│   ├── input_test.rs
│   ├── integration_test.rs
│   ├── ipc_test.rs
│   ├── layer_manager_test.rs
│   ├── layer_test.rs
│   ├── mapping_test.rs
│   ├── window_calc_test.rs
│   └── window_manager_test.rs
└── examples/               # 示例配置
    ├── minimal.toml
    ├── test_config.toml
    ├── window_manager.toml
    └── navigation_layer.toml
```

## 开发计划

### Phase 1: Windows 基础架构 ✅ 已完成

- [x] 项目搭建
- [x] 核心数据结构（输入事件、动作、映射规则）
- [x] IPC 通信（Named Pipe → 统一 TCP）
- [x] 配置系统（TOML 格式）
- [x] Windows 输入抓取（Raw Input）
- [x] Windows 输入发送（SendInput）
- [x] 基础映射引擎

### Phase 2: Windows 键盘增强 + 系统托盘 ✅ 已完成

- [x] 键位重映射基础
- [x] 快捷键层系统
- [x] 导航层配置
- [x] 上下文感知
- [x] 快速启动
- [x] 系统托盘客户端

### Phase 3: Windows 窗口管理 ✅ 已完成

- [x] 窗口信息获取
- [x] 窗口操作基础
- [x] 窗口位置预设（借鉴 mrw）
- [x] 窗口切换基础（Alt+`）
- [x] Action 系统集成

### Phase 4: Windows 鼠标增强 ✅ 已完成

- [x] 鼠标事件处理
- [x] 滚轮增强（加速、水平滚动、音量/亮度控制）

> **注**: 不实现鼠标手势功能，使用场景有限且实现复杂

### Phase 5: Windows 完善 ✅ 已完成

- [x] 系统托盘
- [x] 输入捕获（Raw Input + LLKH）
- [x] 配置重载
- [x] 启动项管理（install.ps1 支持）
- [x] 错误处理和日志
- [x] 安装和打包（install.ps1 脚本）
- [x] 窗口预设功能
- [x] 上下文感知快捷键
- [x] 网络通信（TCP + 多实例支持）

### Phase 6: 宏系统 ✅ 已完成

- [x] 宏录制功能（`MacroRecorder`）
- [x] 宏回放功能（`MacroPlayer`）
- [x] 与 Action 系统整合（复用 `Action` 枚举）
- [x] 支持所有动作类型（Key/Mouse/Window/Launch/System/Delay）
- [x] 延迟优化（自动合并短延迟）
- [x] 宏配置持久化
- [x] 修饰键状态跟踪（`MacroStep` 结构）
- [x] 智能过滤单独修饰键事件

### Phase 7: macOS 移植 ⏳ 待实现

### Phase 8: Linux 移植 ⏳ 待实现

## 参考项目

| 项目 | 语言 | 核心特点 | 学习重点 |
|------|------|----------|----------|
| [keymapper](https://github.com/houmain/keymapper) | C++ | 跨平台、客户端-服务端架构 | 架构设计、配置语法、输入处理 |
| [AutoHotkey](https://github.com/AutoHotkey/AutoHotkey) | C++ | 完整脚本语言、强大热键系统 | 热键变体、窗口操作、消息循环 |
| [window-switcher](https://github.com/sigoden/window-switcher) | Rust | 精致窗口切换、GDI+ 界面 | 窗口切换 UI、图标获取、虚拟桌面 |
| **mrw** (个人项目) | Lua/AHK | 简洁窗口管理、循环尺寸调整 | 窗口布局算法、多显示器支持 |

---

## 参考项目详细分析

### 1. keymapper 架构分析

**keymapper** 是一个跨平台的上下文感知键盘重映射工具，采用客户端-服务端架构。

#### 核心架构

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Space                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  keymapper   │  │ keymapperctl │  │   Configuration      │  │
│  │  (Client)    │  │  (Control)   │  │   (keymapper.conf)   │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────────────────┘  │
│         │                 │                                      │
│         └─────────────────┘                                      │
│                   │                                              │
│              IPC Socket                                          │
│                   │                                              │
│  ┌────────────────┴────────────────┐                            │
│  │         keymapperd (Server)      │                            │
│  │  ┌─────────────┐ ┌─────────────┐ │                            │
│  │  │   Stage     │ │ MultiStage  │ │  <- Runtime Core           │
│  │  │  (Mapping)  │ │ (Pipeline)  │ │                            │
│  │  └─────────────┘ └─────────────┘ │                            │
│  └────────────────┬────────────────┘                            │
│                   │                                              │
└───────────────────┼──────────────────────────────────────────────┘
                    │
┌───────────────────┼──────────────────────────────────────────────┐
│                   │           Kernel Space                        │
│  ┌────────────────┴────────────────┐                            │
│  │     Input Device Drivers         │                            │
│  │  ┌─────────┐ ┌─────────┐        │                            │
│  │  │ Keyboard│ │  Mouse  │        │                            │
│  │  └────┬────┘ └────┬────┘        │                            │
│  └───────┼───────────┼─────────────┘                            │
│          │           │                                           │
│  ┌───────┴───────────┴───────┐                                  │
│  │   Event Subsystem          │                                  │
│  │  (evdev/RawInput/IOKit)    │                                  │
│  └────────────────────────────┘                                  │
└─────────────────────────────────────────────────────────────────┘
```

#### 可借鉴的设计

1. **客户端-服务端架构** - 分离配置管理和底层输入处理
2. **多阶段处理** - 支持多阶段按键映射管道
3. **上下文感知** - 根据窗口标题、类名、进程路径动态切换映射
4. **虚拟键系统** - 支持状态切换和层切换
5. **跨平台抽象层** - 统一的设备接口和窗口检测接口

---

### 2. AutoHotkey 架构分析

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

### 3. Window Switcher 架构分析

**Window Switcher** 是一个用 Rust 编写的 Windows 窗口切换增强工具。

#### 核心特点

1. **应用切换** - Alt+Tab 切换不同应用程序
2. **窗口切换** - Alt+` 在同一应用的多个窗口间切换
3. **可视化界面** - 使用 GDI+ 绘制美观的切换界面
4. **系统托盘** - 支持配置和开机启动
5. **虚拟桌面支持** - 可选仅显示当前虚拟桌面的窗口
6. **UWP 应用支持** - 正确显示 Windows Store 应用图标

#### 可借鉴的设计

1. **模块化工具函数** - utils 目录的良好组织
2. **配置驱动** - INI 格式简单易用
3. **图标缓存** - 使用 `HashMap<String, HICON>` 缓存图标
4. **权限适配** - 根据是否管理员选择不同的启动方式
5. **主题适配** - 检测系统主题色，自动切换深色/浅色界面
6. **GDI+ 抗锯齿绘制** - 使用 `GdipSetSmoothingMode` 实现高质量界面

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

---

## 功能实现状态

### 已实现功能 ✅

**窗口管理**
- 窗口居中
- 移动到边缘
- 半屏显示
- 循环调整宽度/高度
- 固定比例窗口
- 原生比例窗口
- 跨显示器移动
- 同进程窗口切换
- 窗口置顶/透明/最小化/最大化/关闭

**键盘增强**
- 键位重映射
- 快捷键层系统
- 导航层（Vim 风格）
- 应用快捷键
- 快速启动

**鼠标增强**
- 鼠标事件捕获
- 滚轮加速
- 水平滚动（Shift + 滚轮）
- 音量控制（RightAlt + 滚轮）
- 亮度控制（RightCtrl + 滚轮）

**系统功能**
- 系统托盘
- 配置热重载
- 自定义托盘图标
- 调试信息/通知
- 多实例支持

**高级功能**
- 窗口预设（保存/恢复窗口布局）
- 上下文感知快捷键（应用专属快捷键）
- 网络通信（TCP + 远程控制）
- 挑战-响应认证
- 实例发现和管理
- **宏录制回放系统** ✅ 已实现

### 待实现功能 ⏳

**跨平台移植**
- macOS 支持
- Linux 支持

---

## 配置参考

完整的配置说明请参考 [CONFIG.md](CONFIG.md)。

开发相关配置示例：

```toml
# 开发调试配置
log_level = "debug"
tray_icon = true
auto_reload = true

[window]
shortcuts = [
    { "Ctrl+Alt+Win+W" = "ShowDebugInfo" },
    { "Ctrl+Alt+Win+Shift+W" = "ShowNotification(wakem, Debug Mode)" },
]
```

---

## 宏系统

宏系统允许用户录制和回放键盘/鼠标操作序列。

### 使用方式

```bash
# 录制宏
wakem record my-macro
# 执行操作...
# 按 Ctrl+Shift+Esc 停止录制

# 播放宏
wakem play my-macro

# 绑定宏到触发键
wakem bind-macro my-macro F1
```

### 核心组件

| 组件 | 文件 | 说明 |
|------|------|------|
| `MacroRecorder` | `src/types/macros.rs` | 录制输入事件 |
| `MacroPlayer` | `src/runtime/macro_player.rs` | 回放宏动作 |
| `MacroStep` | `src/types/macros.rs` | 宏步骤结构 |
| `Action` | `src/types/action.rs` | 统一的动作枚举 |

> **详细文档**: 完整的宏系统文档请参考 [MACROS.md](MACROS.md)。

---

## 预留扩展 API

以下 API 和功能已在代码中定义但尚未完全使用，为未来扩展预留：

### 1. 触发器类型 (`Trigger`)

位置: `src/types/mapping.rs`

| 触发器类型 | 状态 | 说明 |
|-----------|------|------|
| `HotString { trigger: String }` | 预留 | 热字符串（文本扩展），类似 AutoHotkey 的 ::btw::be right back:: |
| `Chord(Vec<Trigger>)` | 预留 | 组合触发（多个按键按顺序），如 `Ctrl,K,C` |
| `Timer { interval_ms: u64 }` | 预留 | 定时触发器，用于定时执行任务 |
| `Always` | 预留 | 总是触发的规则 |

### 2. 鼠标事件处理

位置: `src/runtime/mapper.rs:104`

```rust
InputEvent::Mouse(_) => {
    // 鼠标事件处理（TODO）
    None
}
```

当前鼠标事件仅用于滚轮增强，完整的鼠标映射（如鼠标按钮重映射）尚未实现。

### 3. 通配符匹配

位置: `src/types/mapping.rs:244-255`

```rust
// TODO: 实现完整的通配符匹配
fn wildcard_match(text: &str, pattern: &str) -> bool {
    // 简化实现，实际应该使用更复杂的匹配算法
}
```

当前通配符匹配仅支持简单的 `*` 匹配，完整的 `*` 和 `?` 通配符支持待实现。

### 4. 配置字段

#### 鼠标配置 (`MouseConfig`)
位置: `src/config.rs:465-473`

```rust
pub struct MouseConfig {
    /// 按钮重映射（预留）
    pub button_remap: HashMap<String, String>,
    /// 滚轮设置
    pub wheel: WheelConfig,
}
```

`button_remap` 字段已定义但未实现功能。

#### 自动重载配置
位置: `examples/test_config.toml:10-11`

```toml
# 自动重新加载配置（预留）
auto_reload = true
```

配置项存在但文件监控和自动重载逻辑待完善。

### 5. IPC 消息

位置: `src/ipc/mod.rs`

以下消息类型已定义但部分功能未完全使用：

| 消息 | 状态 | 说明 |
|------|------|------|
| `GetNextKeyInfo` | 预留 | 获取下一个按键信息（用于调试），服务端响应已实现但客户端未调用 |
| `SaveConfig` | 已实现 | 保存配置到文件，可通过 CLI 触发 |
| `RegisterMessageWindow { hwnd: usize }` | 已实现 | 注册消息窗口句柄，用于托盘通知 |

### 6. 层管理 API

位置: `src/types/layer.rs`, `src/runtime/layer_manager.rs`

以下方法已定义但标记为 `#[allow(dead_code)]`：

| 方法 | 位置 | 说明 |
|------|------|------|
| `is_layer_active()` | `layer.rs:145` | 检查层是否激活 |
| `get_active_layers()` | `layer.rs:151` | 获取当前激活的层列表 |
| `clear_active_layers()` | `layer.rs:157` | 清空所有激活的层 |
| `clear_layers()` | `layer_manager.rs:115` | 停用所有层 |

### 7. 映射规则 API

位置: `src/types/mapping.rs`

以下方法已定义但标记为 `#[allow(dead_code)]`：

| 方法 | 行号 | 说明 |
|------|------|------|
| `with_name()` | 31 | 为映射规则设置名称 |
| `with_context()` | 37 | 为映射规则添加上下文条件 |

### 8. 网络配置 API

位置: `src/config.rs:233-241`

```rust
impl NetworkConfig {
    /// 获取实例通信端口
    #[allow(dead_code)]
    pub fn get_port(&self) -> u16 {
        crate::ipc::get_instance_port(self.instance_id)
    }
}
```

`get_port()` 方法已定义但未使用，当前直接使用 `get_bind_address()`。

### 9. 客户端 API

位置: `src/client.rs:50-52`

```rust
#[allow(dead_code)]
pub fn is_connected(&self) -> bool {
    self.client.is_some()
}
```

`is_connected()` 方法已定义但未使用。

### 10. 输入设备 API

位置: `src/platform/windows/input.rs`

以下方法已定义但标记为 `#[allow(dead_code)]`：

| 方法 | 行号 | 说明 |
|------|------|------|
| `update_modifier_state()` | 327 | 更新修饰键状态 |
| `get_modifier_state()` | 337 | 获取当前修饰键状态 |

### 11. 上下文信息 API

位置: `src/types/mapping.rs:233-240`

```rust
#[allow(dead_code)]
pub struct ContextInfo {
    pub window_class: String,
    pub process_name: String,
    pub process_path: String,
    pub window_title: String,
    pub window_handle: isize, // HWND
}
```

`ContextInfo` 结构体已定义，但当前使用 `WindowContext` 替代。

### 12. 配置保存 API

位置: `src/config.rs:78-83`

```rust
#[allow(dead_code)]
pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
    let content = toml::to_string_pretty(self)?;
    std::fs::write(path, content)?;
    Ok(())
}
```

`save_to_file()` 方法已实现，主要通过 `daemon.rs` 中的 `save_config_to_file()` 调用。

---

## 扩展建议

### 短期可实现的扩展

1. **热字符串 (HotString)** - 实现文本扩展功能
2. **鼠标按钮重映射** - 完成 `MouseConfig.button_remap` 功能
3. **完整的通配符匹配** - 支持 `*` 和 `?` 的完整通配符匹配

### 中期扩展

1. **组合触发 (Chord)** - 实现顺序按键触发
2. **定时触发器** - 实现定时任务功能
3. **鼠标手势** - 虽然文档说明不实现，但代码结构已支持扩展

### 长期扩展

1. **脚本引擎** - 类似 AutoHotkey 的脚本语言支持
2. **插件系统** - 支持动态加载扩展
3. **跨平台抽象层** - 为 macOS/Linux 移植做准备
