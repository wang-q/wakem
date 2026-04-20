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
│   │   ├── action.rs
│   │   ├── input.rs
│   │   ├── layer.rs
│   │   └── mapping.rs
│   ├── ipc/                # IPC 通信
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   └── server.rs
│   ├── platform/           # 平台相关
│   │   └── windows/        # Windows 实现
│   │       ├── mod.rs
│   │       ├── input.rs
│   │       ├── hook.rs
│   │       ├── window_manager.rs
│   │       ├── launcher.rs
│   │       ├── context.rs
│   │       ├── output.rs
│   │       └── tray.rs
│   └── runtime/            # 运行时逻辑
│       ├── mod.rs
│       ├── layer_manager.rs
│       └── mapper.rs
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
- [x] IPC 通信（Named Pipe）
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

### Phase 5: Windows 完善 ⏳ 进行中

- [x] 系统托盘
- [x] 输入捕获（Raw Input + LLKH）
- [x] 配置重载
- [x] 启动项管理（install.ps1 支持）
- [x] 错误处理和日志
- [x] 安装和打包（install.ps1 脚本）

### Phase 6: macOS 移植 ⏳ 待实现

### Phase 7: Linux 移植 ⏳ 待实现

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

### 待实现功能 ⏳

**鼠标增强**
- 鼠标事件处理
- 滚轮增强

**系统完善**
- 启动项管理
- 安装和打包

**跨平台移植**
- macOS 支持
- Linux 支持

---

## 配置参考

用户配置请参考 [CONFIG.md](CONFIG.md)。

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

### 窗口预设配置示例

```toml
[window]
# 自动应用预设（当窗口创建或激活时）
auto_apply_preset = true

# 定义窗口预设
[[window.presets]]
name = "browser"
process_name = "chrome.exe"
x = 100
y = 100
width = 1200
height = 800

[[window.presets]]
name = "editor"
process_name = "code.exe"
executable_path = "C:\\Program Files\\Microsoft VS Code\\Code.exe"
x = 200
y = 50
width = 1400
height = 900

# 预设快捷键
[window.shortcuts]
"Ctrl+Alt+S" = "SavePreset(main)"
"Ctrl+Alt+L" = "LoadPreset(main)"
"Ctrl+Alt+A" = "ApplyPreset"
```

### 上下文感知快捷键配置示例

```toml
# Chrome 浏览器专属快捷键
[[keyboard.context_mappings]]
context = { process_name = "chrome.exe" }
"CapsLock" = "Backspace"
"Ctrl+H" = "ShowNotification(Browser, History)"
"Ctrl+J" = "ShowNotification(Browser, Downloads)"

# VS Code 专属快捷键
[[keyboard.context_mappings]]
context = { process_name = "code.exe" }
"CapsLock" = "Esc"
"Ctrl+P" = "ShowNotification(VSCode, Quick Open)"
"Ctrl+Shift+F" = "ShowNotification(VSCode, Search)"

# 使用通配符匹配多个编辑器
[[keyboard.context_mappings]]
context = { process_name = "*edit*.exe" }
"Ctrl+S" = "ShowNotification(Editor, Save)"

# 窗口标题匹配（如 YouTube）
[[keyboard.context_mappings]]
context = { window_title = "*YouTube*" }
"Space" = "ShowNotification(YouTube, Play/Pause)"

# 可执行文件路径匹配
[[keyboard.context_mappings]]
context = { executable_path = "C:\\Program Files\\JetBrains\\*" }
"Ctrl+Shift+A" = "ShowNotification(JetBrains, Find Action)"
```

**说明**：
- `process_name`: 进程名匹配，支持通配符 `*` 和 `?`
- `window_class`: 窗口类名匹配
- `window_title`: 窗口标题匹配
- `executable_path`: 可执行文件路径匹配
- 上下文规则优先级高于全局规则

---

## 预留 API 清单

以下 API 已定义但未在当前版本中使用，为未来功能预留：

### 上下文感知系统 ✅ 已实现

| API | 说明 | 状态 |
|-----|------|------|
| `ContextCondition` | 上下文条件构建器 | ✅ 已实现 |
| `ContextCondition::with_window_class()` | 窗口类名匹配 | ✅ 已实现 |
| `ContextCondition::with_process_name()` | 进程名匹配 | ✅ 已实现 |
| `ContextCondition::with_window_title()` | 窗口标题匹配 | ✅ 已实现 |
| `ContextInfo` | 当前上下文信息 | ✅ 已实现 (WindowContext) |
| `MappingRule::with_context()` | 添加上下文条件 | ✅ 已实现 |
| `KeyMapper::process_event_with_context()` | 上下文感知事件处理 | ✅ 已实现 |
| `keyboard.context_mappings` | 配置支持 | ✅ 已实现 |

### 层系统扩展

| API | 说明 | 计划用途 |
|-----|------|----------|
| `LayerStack::is_layer_active()` | 检查层是否激活 | 查询层状态 |
| `LayerStack::get_active_layers()` | 获取所有激活层 | 显示当前激活层 |
| `LayerStack::clear_active_layers()` | 清除所有激活层 | 重置层状态 |
| `LayerManager::get_active_layers()` | 获取激活层列表 | 显示层状态 |
| `LayerManager::is_layer_active()` | 检查层是否激活 | 查询层状态 |
| `LayerManager::clear_layers()` | 清除所有层 | 重置层状态 |

### 运行时管理

| API | 说明 | 计划用途 |
|-----|------|----------|
| `KeyMapper::set_window_manager()` | 设置窗口管理器 | 动态切换窗口管理器 |
| `KeyMapper::set_tray_icon()` | 设置托盘图标 | 动态更新托盘 |
| `KeyMapper::add_rule()` | 添加单条规则 | 运行时动态添加映射 |
| `KeyMapper::clear()` | 清除所有规则 | 重置映射 |
| `KeyMapper::set_enabled()` / `is_enabled()` | 启用/禁用映射 | 全局开关 |
| `KeyMapper::add_simple_remap()` | 添加简单重映射 | 简化 API |

### 配置系统扩展

| API | 说明 | 计划用途 |
|-----|------|----------|
| `Config::save_to_file()` | 保存配置到文件 | 配置持久化 |
| `WindowPosition` | 窗口位置预设 | 保存/恢复窗口布局 |
| `MouseConfig` | 鼠标配置 | 鼠标增强功能 |
| `WheelConfig` | 滚轮配置 | 滚轮增强（加速、水平滚动、音量/亮度控制） |

### IPC 系统扩展

| API | 说明 | 计划用途 |
|-----|------|----------|
| `IpcError::ConnectionRefused` | 连接被拒绝错误 | 错误处理 |
| `DEFAULT_PORT` | 默认 TCP 端口 | 网络通信备用 |
| `IpcClient::with_pipe_name()` | 自定义管道名 | 多实例支持 |
| `IpcClient::close()` | 关闭连接 | 优雅关闭 |
| `IpcServer::with_pipe_name()` | 自定义管道名 | 多实例支持 |
| `IpcServer::stop()` | 停止服务器 | 优雅关闭 |

### Windows 平台扩展

| API | 说明 | 计划用途 |
|-----|------|----------|
| `WindowContext` | 窗口上下文 | 上下文感知基础 |
| `WindowContext::get_current()` | 获取当前上下文 | 自动检测活动窗口 |
| `WindowContext::matches()` | 匹配窗口条件 | 上下文过滤 |
| `KeyboardHook` | 低级键盘钩子 | 备用输入捕获方式 |
| `RawInputDevice::get_modifier_state()` | 获取修饰键状态 | 查询当前修饰键 |
| `TrayIcon::set_active()` | 设置激活状态 | 视觉反馈 |
| `Launcher::create_action()` | 创建启动动作 | 程序化创建 |
| `Launcher::parse_command()` | 解析命令字符串 | 动态解析 |

### 输入事件扩展

| API | 说明 | 计划用途 |
|-----|------|----------|
| `KeyEvent::with_modifiers()` | 设置修饰键 | 构建事件 |
| `KeyEvent::injected()` | 标记为注入事件 | 模拟输入 |
| `KeyEvent::is_modifier()` | 检查修饰键 | 特殊处理 |
| `MouseEvent::with_modifiers()` | 设置修饰键 | 构建事件 |
| `MouseEvent::injected()` | 标记为注入事件 | 模拟输入 |
| `InputEvent::timestamp()` | 获取时间戳 | 时序分析 |
| `InputEvent::is_injected()` | 检查是否注入 | 过滤模拟输入 |

> **注**: 鼠标手势功能已移除，使用场景有限且实现复杂

### 动作系统扩展

| API | 说明 | 计划用途 |
|-----|------|----------|
| `KeyAction::press_from_event()` | 从事件创建 Press | 事件转发 |
| `KeyAction::release_from_event()` | 从事件创建 Release | 事件转发 |
| `KeyAction::combo()` | 创建组合键 | 复杂快捷键 |
| `Action::mouse()` | 创建鼠标动作 | 鼠标控制 |
| `Action::launch()` | 创建启动动作 | 程序启动 |
| `Action::sequence()` | 创建动作序列 | 批量执行 |

### 修饰键状态扩展

| API | 说明 | 计划用途 |
|-----|------|----------|
| `ModifierState::is_empty()` | 检查是否为空 | 验证 |
| `ModifierState::from_virtual_key()` | 从虚拟键创建 | 事件解析 |
| `ModifierState::merge()` | 合并修饰键状态 | 组合状态 |

### 消息窗口扩展

| API | 说明 | 计划用途 |
|-----|------|----------|
| `MessageWindow::new()` | 创建消息窗口 | 多窗口支持 |
| `MessageWindow::hwnd()` | 获取窗口句柄 | 外部集成 |
| `MessageWindow::tray_icon()` | 获取托盘图标 | 外部控制 |

### 客户端扩展

| API | 说明 | 计划用途 |
|-----|------|----------|
| `DaemonClient::close()` | 关闭客户端 | 优雅关闭 |

---

## 预留功能规划

基于上述预留 API，未来可能实现的功能：

1. **上下文感知快捷键** (Phase 6) ✅ 已实现
   - 根据当前应用自动切换映射
   - 为特定应用定义专属快捷键
   - 配置示例见下文"上下文感知快捷键配置示例"

2. **多实例支持** (Phase 6)
   - 运行多个 wakem 实例
   - 每个实例独立配置

6. **网络通信** (Phase 6)
   - TCP 备用通信方式
   - 远程控制支持
