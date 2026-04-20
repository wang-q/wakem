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
- 多实例支持

**高级功能**
- 窗口预设（保存/恢复窗口布局）
- 上下文感知快捷键（应用专属快捷键）
- 网络通信（TCP + 远程控制）
- 挑战-响应认证
- 实例发现和管理

### 待实现功能 ⏳

**跨平台移植**
- macOS 支持
- Linux 支持

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

### 网络通信配置示例

```toml
# 启用网络通信（用于远程控制）
[network]
enabled = true
bind_address = "0.0.0.0:57427"
auth_key = "your-secret-key-here"
```

**安全特性**：
- 自动拒绝外网连接（只允许 RFC 1918 内网地址）
- 挑战-响应认证（HMAC-SHA256）
- 密钥不在网络上传输

**远程控制示例**：

```bash
# 在被控制的电脑上启动 wakemd（配置好 auth_key）
wakemd

# 在另一台电脑上查看远程状态
wakem --host 192.168.1.100 --auth-key "your-secret-key-here" status

# 重新加载远程配置
wakem --host 192.168.1.100 --auth-key "your-secret-key-here" reload

# 启用/禁用远程映射
wakem --host 192.168.1.100 --auth-key "your-secret-key-here" enable
wakem --host 192.168.1.100 --auth-key "your-secret-key-here" disable
```

### 多实例配置示例

```toml
# 实例0配置（默认）: ~/.wakem.toml
[network]
enabled = true
instance_id = 0
auth_key = "instance0-secret"
```

```toml
# 实例1配置: ~/.wakem-instance1.toml
[network]
enabled = true
instance_id = 1
auth_key = "instance1-secret"
```

**端口分配**：
- 实例0: 127.0.0.1:57427
- 实例1: 127.0.0.1:57428
- 实例2: 127.0.0.1:57429
- ...

**使用示例**：

```bash
# 启动实例0（默认）
wakemd

# 启动实例1
wakemd --instance 1

# 查看运行中的实例
wakem instances

# 连接到实例1
wakem --instance 1 status
wakem --instance 1 reload

# 启动实例1的托盘
wakem --instance 1
```

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

### 网络通信系统 ✅ 已实现

| API | 说明 | 状态 |
|-----|------|------|
| `NetworkConfig` | 网络配置 | ✅ 已实现 |
| `TcpIpcServer` | TCP 服务端 | ✅ 已实现 |
| `TcpIpcClient` | TCP 客户端 | ✅ 已实现 |
| `auth::generate_challenge()` | 生成认证挑战 | ✅ 已实现 |
| `auth::compute_response()` | 计算认证响应 | ✅ 已实现 |
| `security::is_private_ip()` | 内网 IP 检查 | ✅ 已实现 |
| `DaemonClient::connect_tcp()` | TCP 连接 | ✅ 已实现 |

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

## 未使用代码总结

以下代码已实现但当前未被主流程使用，使用 `#[allow(dead_code)]` 标记保留。这些代码为未来功能扩展提供了基础。

### 1. 层系统扩展 API

**位置**: `src/types/layer.rs`, `src/runtime/layer_manager.rs`

| API | 说明 | 启用条件 |
|-----|------|----------|
| `LayerStack::is_layer_active()` | 检查层是否激活 | 添加层状态查询功能 |
| `LayerStack::get_active_layers()` | 获取所有激活层 | 添加层状态显示 |
| `LayerStack::clear_active_layers()` | 清除所有激活层 | 添加层重置功能 |
| `LayerManager::get_active_layers()` | 获取激活层列表 | 添加层状态查询 |
| `LayerManager::is_layer_active()` | 检查层是否激活 | 添加层状态查询 |
| `LayerManager::clear_layers()` | 清除所有层 | 添加层重置功能 |

**启用建议**: 当需要添加层状态显示或层管理 UI 时启用。

### 2. 运行时动态管理 API

**位置**: `src/runtime/mapper.rs`

| API | 说明 | 启用条件 |
|-----|------|----------|
| `KeyMapper::set_window_manager()` | 动态设置窗口管理器 | 支持运行时切换窗口管理器 |
| `KeyMapper::set_tray_icon()` | 动态设置托盘图标 | 支持运行时更新托盘 |
| `KeyMapper::add_rule()` | 运行时添加单条规则 | 支持动态添加映射 |
| `KeyMapper::clear()` | 清除所有规则 | 支持重置映射 |
| `KeyMapper::set_enabled()` / `is_enabled()` | 启用/禁用映射 | 添加全局开关功能 |
| `KeyMapper::add_simple_remap()` | 简化 API 添加重映射 | 添加简化配置接口 |

**启用建议**: 当需要添加运行时配置修改功能（如通过托盘菜单动态添加映射）时启用。

### 3. 配置持久化 API ✅ 已启用

**位置**: `src/config.rs`, `src/daemon.rs`, `src/client.rs`, `src/main.rs`

| API | 说明 | 状态 |
|-----|------|------|
| `Config::save_to_file()` | 保存配置到文件 | ✅ 已启用 |
| `ServerState::save_config_to_file()` | 守护进程保存配置 | ✅ 已启用 |
| `DaemonClient::save_config()` | 客户端保存配置 | ✅ 已启用 |
| `cmd_save()` | 命令行保存命令 | ✅ 已启用 |
| `Message::SaveConfig` | IPC 保存消息 | ✅ 已启用 |
| `NetworkConfig::get_port()` | 获取实例端口 | 需要直接访问端口时 |

**使用方式**:
```bash
# 保存当前配置到文件
wakem save

# 保存指定实例的配置
wakem --instance 1 save
```

**典型应用场景**:
- 保存当前窗口状态到配置文件
- 保存动态添加的映射规则
- 保存修改后的层配置

### 4. IPC 系统扩展 API

**位置**: `src/ipc/client.rs`, `src/ipc/server.rs`, `src/ipc/discovery.rs`

| API | 说明 | 启用条件 |
|-----|------|----------|
| `IpcClient::is_connected()` | 检查连接状态 | 需要查询连接状态时 |
| `IpcClient::close()` | 优雅关闭连接 | 需要显式关闭连接时 |
| `IpcServer::stop()` | 停止服务器 | 需要运行时停止服务器时 |
| `InstanceInfo::port` | 实例端口号 | 需要直接访问端口时 |
| `find_first_active_instance()` | 查找第一个活跃实例 | 自动连接功能 |
| `is_instance_active()` | 检查指定实例是否活跃 | 实例健康检查 |

**启用建议**: 当需要添加连接状态监控、自动重连或实例健康检查功能时启用。

### 5. Windows 平台备用方案

**位置**: `src/platform/windows/hook.rs`

| API | 说明 | 启用条件 |
|-----|------|----------|
| `KeyboardHook` | 低级键盘钩子 | Raw Input 出现问题时备用 |
| `KeyboardHook::new()` | 创建钩子 | 启用钩子方案时 |
| `KeyboardHook::run_message_loop()` | 运行消息循环 | 启用钩子方案时 |
| `KeyboardHook::uninstall()` | 卸载钩子 | 启用钩子方案时 |
| `is_hook_installed()` | 检查钩子状态 | 启用钩子方案时 |

**启用建议**: 当 Raw Input 在某些场景下工作不正常时，可切换到低级键盘钩子方案。

### 6. 输入事件构建 API

**位置**: `src/types/input.rs`, `src/types/mod.rs`

| API | 说明 | 启用条件 |
|-----|------|----------|
| `KeyEvent::with_modifiers()` | 设置修饰键 | 构建模拟事件时 |
| `KeyEvent::injected()` | 标记为注入事件 | 需要区分模拟输入时 |
| `KeyEvent::is_modifier()` | 检查修饰键 | 特殊键处理时 |
| `KeyEvent::modifier_identifier()` | 获取修饰键标识 | 显示修饰键状态时 |
| `MouseEvent::with_modifiers()` | 设置修饰键 | 构建模拟鼠标事件时 |
| `MouseEvent::injected()` | 标记为注入事件 | 需要区分模拟输入时 |
| `MouseEvent::is_button_up()` | 检查按钮释放 | 鼠标事件处理时 |
| `InputEvent::timestamp()` | 获取时间戳 | 时序分析功能 |
| `ModifierState::is_empty()` | 检查是否为空 | 验证修饰键状态时 |
| `ModifierState::from_virtual_key()` | 从虚拟键创建 | 解析事件时 |
| `ModifierState::merge()` | 合并修饰键状态 | 组合多个状态时 |

**启用建议**: 当需要构建模拟输入事件、显示修饰键状态或进行时序分析时启用。

### 7. 动作系统构建 API

**位置**: `src/types/action.rs`

| API | 说明 | 启用条件 |
|-----|------|----------|
| `KeyAction::press_from_event()` | 从事件创建 Press | 事件转发时 |
| `KeyAction::release_from_event()` | 从事件创建 Release | 事件转发时 |
| `KeyAction::combo()` | 创建组合键 | 构建复杂快捷键时 |
| `Action::mouse()` | 创建鼠标动作 | 程序化创建鼠标动作 |
| `Action::launch()` | 创建启动动作 | 程序化创建启动动作 |
| `Action::sequence()` | 创建动作序列 | 批量执行动作时 |
| `Action::is_none()` | 检查是否为空操作 | 验证动作时 |

**启用建议**: 当需要程序化构建动作（如通过 UI 配置快捷键）时启用。

### 8. 窗口管理扩展 API

**位置**: `src/platform/windows/window_manager.rs`

| API | 说明 | 启用条件 |
|-----|------|----------|
| `WindowFrame::to_rect()` | 转换为 RECT | 需要 Windows API 交互时 |
| `WindowInfo::is_minimized` | 是否最小化 | 窗口状态检测 |
| `WindowInfo::is_maximized` | 是否最大化 | 窗口状态检测 |

**启用建议**: 当需要更复杂的窗口状态检测或与 Windows API 深度交互时启用。

### 9. 窗口预设管理 API

**位置**: `src/platform/windows/window_preset.rs`

| API | 说明 | 启用条件 |
|-----|------|----------|
| `WindowPresetManager::get_presets()` | 获取所有预设 | 显示预设列表时 |
| `WindowPresetManager::get_presets_mut()` | 获取可变更预设 | 修改预设时 |
| `WindowPresetManager::find_matching_preset()` | 查找匹配预设 | 自动应用预设时 |
| `WindowPresetManager::remove_preset()` | 删除预设 | 管理预设时 |

**启用建议**: 当需要添加窗口预设管理 UI（如保存/加载/删除预设）时启用。

### 10. 上下文感知系统 ✅ 已统一

**位置**: `src/types/mapping.rs`, `src/config.rs`, `src/runtime/mapper.rs`

**说明**: `types::ContextCondition` 已成为标准 API，`config.rs` 已改用此版本。

#### 核心 API（已启用）

| API | 说明 | 状态 |
|-----|------|------|
| `ContextCondition` | 上下文条件结构体 | ✅ 已启用 |
| `ContextCondition::new()` | 创建上下文条件 | ✅ 已启用 |
| `ContextCondition::with_window_class()` | 窗口类名匹配 | ✅ 已启用 |
| `ContextCondition::with_process_name()` | 进程名匹配 | ✅ 已启用 |
| `ContextCondition::with_window_title()` | 窗口标题匹配 | ✅ 已启用 |
| `ContextCondition::with_executable_path()` | 可执行路径匹配 | ✅ 已启用 |
| `ContextCondition::matches()` | 检查上下文匹配 | ✅ 已启用 |
| `ContextInfo` | 上下文信息结构体 | ✅ 已启用 |

#### 扩展 API（未使用）

| API | 说明 | 启用条件 |
|-----|------|----------|
| `WindowContext::matches()` | 匹配窗口条件 | 自定义上下文匹配逻辑时 |
| `MappingRule::with_name()` | 设置规则名称 | 添加规则命名时 |
| `MappingRule::with_context()` | 添加上下文条件 | 程序化构建规则时 |
| `MappingRule::matches()` | 检查规则匹配 | 自定义匹配逻辑时 |
| `wildcard_match()` | 通配符匹配 | 需要通配符匹配时 |

**使用示例**:
```rust
// 创建上下文条件
let cond = ContextCondition::new()
    .with_process_name("chrome.exe")
    .with_window_title("*YouTube*");

// 检查是否匹配
let matches = cond.matches(
    "chrome.exe",           // process_name
    "Chrome_WidgetWin_1",   // window_class
    "YouTube - Google Chrome", // window_title
    Some("C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe") // executable_path
);
```

**启用建议**: 
- 核心 API 已启用，用于配置文件解析和运行时匹配
- 扩展 API 用于程序化构建规则（如通过 UI 配置）

### 11. 系统托盘扩展 API

**位置**: `src/platform/windows/tray.rs`

| API | 说明 | 启用条件 |
|-----|------|----------|
| `TrayIcon::set_active()` | 设置激活状态 | 需要视觉反馈时 |

**启用建议**: 当需要添加激活状态视觉反馈（如切换图标颜色）时启用。

### 12. 程序启动扩展 API

**位置**: `src/platform/windows/launcher.rs`

| API | 说明 | 启用条件 |
|-----|------|----------|
| `Launcher::create_action()` | 创建启动动作 | 程序化构建启动动作 |
| `Launcher::parse_command()` | 解析命令字符串 | 支持命令行语法 |

**启用建议**: 当需要程序化构建启动动作（如通过 UI 配置快速启动）时启用。

### 13. 消息窗口扩展 API

**位置**: `src/window.rs`

| API | 说明 | 启用条件 |
|-----|------|----------|
| `MessageWindow::new()` | 创建消息窗口 | 多窗口支持 |
| `MessageWindow::hwnd()` | 获取窗口句柄 | 外部集成 |
| `MessageWindow::tray_icon()` | 获取托盘图标 | 外部控制托盘 |

**启用建议**: 当需要多窗口支持或外部系统集成时启用。

### 14. 守护进程状态字段

**位置**: `src/daemon.rs`

| 字段 | 说明 | 启用条件 |
|------|------|----------|
| `ServerState::window_manager` | 窗口管理器 | 启用窗口管理功能 |

**启用建议**: `window_manager` 已初始化但未在事件处理中使用，当需要添加窗口管理相关功能时启用。

---

## 如何启用未使用代码

当需要启用某个功能时，按以下步骤操作：

1. **移除 `#[allow(dead_code)]` 标记**（可选，保留也无害）
2. **在 daemon.rs 或 client.rs 中添加调用逻辑**
3. **添加对应的配置支持**（如需要）
4. **添加对应的命令行支持**（如需要）
5. **更新文档**

### 示例：启用层状态查询功能

```rust
// 1. 在 daemon.rs 中添加消息处理
async fn handle_message(&self, message: Message) -> Message {
    match message {
        // ... 现有处理
        Message::GetActiveLayers => {
            let layers = self.layer_manager.lock().await.get_active_layers();
            Message::ActiveLayersResponse { layers }
        }
    }
}

// 2. 在 ipc/mod.rs 中添加消息类型
pub enum Message {
    // ... 现有消息
    GetActiveLayers,
    ActiveLayersResponse { layers: Vec<String> },
}

// 3. 在 client.rs 中添加客户端 API
pub async fn get_active_layers(&mut self) -> Result<Vec<String>> {
    let response = self.send_receive(&Message::GetActiveLayers).await?;
    match response {
        Message::ActiveLayersResponse { layers } => Ok(layers),
        _ => Err(anyhow::anyhow!("Unexpected response")),
    }
}

// 4. 在 main.rs 中添加命令行支持
async fn cmd_layers() -> Result<()> {
    let mut client = DaemonClient::new().await?;
    let layers = client.get_active_layers().await?;
    println!("Active layers: {:?}", layers);
    Ok(())
}
```

---

## 代码统计

- **总未使用 API 数量**: 约 60+
- **层系统扩展**: 6 个
- **运行时管理**: 6 个
- **配置持久化**: 2 个
- **IPC 扩展**: 6 个
- **Windows 平台**: 5 个
- **输入事件**: 9 个
- **动作系统**: 7 个
- **修饰键状态**: 3 个
- **窗口管理**: 3 个
- **窗口预设**: 4 个
- **上下文感知**: 11 个
- **系统托盘**: 1 个
- **程序启动**: 2 个
- **消息窗口**: 3 个
- **守护进程字段**: 1 个

这些代码为未来功能扩展提供了坚实的基础，可以根据需求逐步启用。
