# wakem - Window Adjust, Keyboard Enhance, Mouse enhance

一个跨平台的窗口管理、键盘增强、鼠标增强工具。初期先在 Windows 平台实现，后续扩展到 macOS 和 Linux。

## 功能规划

### 1. 窗口调整 (Window Adjust)

| 功能      | 描述                                 | 参考实现                        |
| ------- | ---------------------------------- | --------------------------- |
| 窗口移动    | 快捷键移动窗口到屏幕各位置（左半、右半、上半、下半、四角、中心等）  | window-switcher, AutoHotkey |
| 窗口调整大小  | 快捷键调整窗口大小（最大化、最小化、1/2屏、1/3屏、1/4屏等） | AutoHotkey                  |
| 窗口切换    | Alt-Tab 增强、同应用窗口切换（Alt+\`）、窗口预览    | window-switcher             |
| 多显示器支持  | 窗口跨显示器移动、每个显示器独立布局                 | AutoHotkey                  |
| 窗口置顶/透明 | 快捷键设置窗口置顶或调整透明度                    | AutoHotkey                  |
| 虚拟桌面    | 快捷键切换/移动窗口到不同虚拟桌面                  | window-switcher             |

### 2. 键盘增强 (Keyboard Enhance)

| 功能    | 描述                                     | 参考实现                  |
| ----- | -------------------------------------- | --------------------- |
| 键位重映射 | CapsLock 改 Backspace/Esc、交换 Ctrl/Alt 等 | keymapper, AutoHotkey |
| 快捷键层  | 按住特定键（如 CapsLock/右Alt）切换快捷键层           | keymapper             |
| 方向键层  | CapsLock + I/J/K/L 作为方向键               | keymapper             |
| 文本扩展  | 输入缩写自动展开（如 `;date` 展开为当前日期）            | AutoHotkey            |
| 应用快捷键 | 为特定应用定义专属快捷键                           | keymapper, AutoHotkey |
| 快速启动  | 快捷键启动常用应用（如 Win+C 启动终端）                | keymapper             |

### 3. 鼠标增强 (Mouse Enhance)

| 功能    | 描述                     | 参考实现       |
| ----- | ---------------------- | ---------- |
| 鼠标手势  | 按住右键画手势执行操作（如关闭窗口、刷新等） | AutoHotkey |
| 滚轮增强  | 滚轮在标签页/音量/亮度间切换        | AutoHotkey |
| 按键重映射 | 鼠标侧键自定义功能              | keymapper  |
| 边缘触发  | 鼠标移到屏幕边缘触发操作（如显示任务视图）  | AutoHotkey |
| 快速滚动  | 加速滚动、平滑滚动              | AutoHotkey |

### 4. 其他功能

| 功能    | 描述             | 参考实现            |
| ----- | -------------- | --------------- |
| 配置热重载 | 修改配置后自动生效      | keymapper       |
| 上下文感知 | 根据当前应用自动切换配置   | keymapper       |
| 系统托盘  | 图形界面管理、快速启用/禁用 | window-switcher |
| 命令行控制 | 通过命令行控制各项功能    | keymapper       |
| 可视化界面 | 窗口切换时显示应用图标预览  | window-switcher |

***

## 技术方案

### 架构设计

参考 [keymapper](https://github.com/houmain/keymapper) 的客户端-服务端架构：

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Space                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │   wakem      │  │  wakemctl    │  │   wakem.conf         │  │
│  │  (Client)    │  │  (Control)   │  │   (Configuration)    │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────────────────┘  │
│         │                 │                                      │
│         └─────────────────┘                                      │
│                   │                                              │
│              IPC Socket                                          │
│                   │                                              │
│  ┌────────────────┴────────────────┐                            │
│  │         wakemd (Server)          │                            │
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
 │  │   Platform Input API      │                                  │
 │  │   (RawInput/IOKit/evdev)  │                                  │
 │  └────────────────────────────┘                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 技术选型

| 组件   | Windows                  | macOS           | Linux              | 说明                |
| ---- | ------------------------ | --------------- | ------------------ | ----------------- |
| 输入抓取 | Raw Input / Interception | IOKit / CGEvent | evdev / libinput   | 平台原生方案            |
| 输入发送 | SendInput                | CGEventPost     | XTest / uinput     | 平台原生方案            |
| 窗口管理 | Win32 API                | Cocoa / Carbon  | X11 / Wayland      | 参考各平台实现           |
| 配置格式 | 自定义 DSL                  | 自定义 DSL         | 自定义 DSL            | 类似 keymapper.conf |
| 进程通信 | Named Pipe               | Unix Socket     | Unix Socket        | keymapper 方案      |
| 界面绘制 | GDI+ / Direct2D          | Core Graphics   | Cairo / OpenGL     | 平台原生方案            |
| 系统托盘 | Win32 API                | Cocoa           | GTK / AppIndicator | 平台原生方案            |

### 关键技术借鉴

#### 1. 从 keymapper 学习

* **配置解析**：上下文感知的配置语法

* **客户端-服务端架构**：分离 UI 和核心逻辑

* **输入处理管道**：多阶段映射处理

* **热重载机制**：配置变更自动生效

#### 2. 从 AutoHotkey 学习

* **热键系统**：修饰键组合、热键变体

* **窗口操作**：全面的 Win32 API 封装

* **消息循环**：`MsgSleep` 替代 `Sleep`

* **脚本执行**：解释执行流程

#### 3. 从 window-switcher 学习

* **窗口切换 UI**：GDI+ 绘制图标预览界面

* **图标获取**：UWP 应用图标解析

* **虚拟桌面**：DWM  cloak 检测

* **权限处理**：管理员/普通用户启动适配

* **代码组织**：清晰的 utils 模块化

***

## 目录结构

```
wakem/
├── src/
│   ├── client/               # 客户端
│   │   ├── main.cpp          # 入口
│   │   ├── ConfigFile.cpp    # 配置管理
│   │   ├── ServerPort.cpp    # 与服务端通信
│   │   ├── ControlPort.cpp   # 命令行控制
│   │   ├── TrayIcon.cpp      # 系统托盘（平台通用接口）
│   │   └── platform/         # 平台特定实现
│   │       ├── windows/      # Windows 实现
│   │       │   ├── FocusedWindowWin32.cpp
│   │       │   └── TrayIconWin32.cpp
│   │       ├── macos/        # macOS 实现
│   │       │   ├── FocusedWindowCarbon.cpp
│   │       │   └── TrayIconCocoa.mm
│   │       └── linux/        # Linux 实现
│   │           ├── FocusedWindowX11.cpp
│   │           ├── FocusedWindowWayland.cpp
│   │           └── TrayIconGTK.cpp
│   ├── server/               # 服务端
│   │   ├── main.cpp          # 入口
│   │   ├── ServerState.cpp   # 状态管理
│   │   ├── runtime/          # 运行时核心（跨平台）
│   │   │   ├── Stage.cpp
│   │   │   ├── MultiStage.cpp
│   │   │   └── KeyState.cpp
│   │   └── platform/         # 平台特定实现
│   │       ├── windows/
│   │       │   ├── InputDeviceWin32.cpp
│   │       │   └── OutputDeviceWin32.cpp
│   │       ├── macos/
│   │       │   ├── InputDeviceMacOS.cpp
│   │       │   └── OutputDeviceMacOS.cpp
│   │       └── linux/
│   │           ├── InputDeviceLinux.cpp
│   │           └── OutputDeviceLinux.cpp
│   ├── config/               # 配置解析（跨平台）
│   │   ├── Parser.cpp
│   │   ├── Expression.cpp
│   │   └── Action.cpp
│   ├── window/               # 窗口管理（平台通用接口）
│   │   ├── WindowManager.h
│   │   ├── WindowSwitcher.h
│   │   └── platform/
│   │       ├── windows/WindowManagerWin32.cpp
│   │       ├── macos/WindowManagerMacOS.mm
│   │       └── linux/WindowManagerLinux.cpp
│   ├── ui/                   # 界面绘制（平台通用接口）
│   │   ├── Painter.h
│   │   └── platform/
│   │       ├── windows/PainterGDI.cpp
│   │       ├── macos/PainterCoreGraphics.mm
│   │       └── linux/PainterCairo.cpp
│   ├── common/               # 共享组件（跨平台）
│   │   ├── Message.h         # 通信协议
│   │   ├── Utils.cpp
│   │   └── Platform.h        # 平台抽象接口
│   └── control/              # 控制工具
│       └── main.cpp
├── docs/                     # 文档
│   ├── keymapper-complete-guide.md
│   ├── autohotkey-analysis.md
│   └── window-switcher-analysis.md
├── wakem.conf                # 示例配置
├── Cargo.toml
└── README.md
```

***

## 配置文件示例

```conf
# wakem.conf - 窗口管理、键盘增强、鼠标增强配置

# ============================================
# 全局设置
# ============================================
log_level = info
tray_icon = true
auto_reload = true

# ============================================
# 窗口调整 (Window Adjust)
# ============================================

# 窗口位置快捷键
[window.position]
# Win + Alt + 方向键移动窗口
Meta{A}Left  >> MoveWindow(LeftHalf)
Meta{A}Right >> MoveWindow(RightHalf)
Meta{A}Up    >> MoveWindow(TopHalf)
Meta{A}Down  >> MoveWindow(BottomHalf)

# Win + Alt + 数字键调整大小
Meta{A}1 >> ResizeWindow(1/2)
Meta{A}2 >> ResizeWindow(1/3)
Meta{A}3 >> ResizeWindow(2/3)
Meta{A}4 >> ResizeWindow(1/4)

# 窗口切换
[window.switch]
# Alt + ` 同应用窗口切换（参考 window-switcher）
Alt{Grave} >> SwitchWindows()

# Alt + Tab 应用切换（增强版）
Alt{Tab} >> SwitchApps()

# 多显示器
[window.monitor]
Meta{Shift}Left  >> MoveToMonitor(Prev)
Meta{Shift}Right >> MoveToMonitor(Next)

# 窗口属性
[window.property]
Meta{T} >> ToggleTopmost()
Meta{O} >> ToggleOpacity()

# ============================================
# 键盘增强 (Keyboard Enhance)
# ============================================

# 键位重映射
[keyboard.remap]
CapsLock >> Backspace
RightAlt >> Control

# 快捷键层 - 按住 CapsLock 时
[layer:navigate]
CapsLock{
  H >> Left
  J >> Down
  K >> Up
  L >> Right
  I >> Home
  O >> End
  U >> PageUp
  P >> PageDown
}

# 应用专属快捷键
[context:firefox]
window_class = "MozillaWindowClass"

[context:firefox.keyboard]
# 在 Firefox 中 Ctrl+J 打开下载
Meta{J} >> !Ctrl{J}

# 快速启动
[launch]
Meta{C} >> Launch("wt.exe")
Meta{E} >> Launch("explorer.exe")
Meta{T} >> Launch("wt.exe")

# ============================================
# 鼠标增强 (Mouse Enhance)
# ============================================

# 鼠标手势
[mouse.gesture]
# 右键画圈关闭窗口
RightButton{Circle} >> CloseWindow()
# 右键上滑刷新
RightButton{Up} >> Refresh()

# 滚轮增强
[mouse.wheel]
# 在任务栏滚轮调节音量
# 在标题栏滚轮调节透明度

# 按键重映射
[mouse.button]
XButton1 >> BrowserBack
XButton2 >> BrowserForward

# ============================================
# 虚拟桌面
# ============================================

[virtual_desktop]
Meta{1} >> SwitchDesktop(1)
Meta{2} >> SwitchDesktop(2)
Meta{3} >> SwitchDesktop(3)
Meta{4} >> SwitchDesktop(4)
Meta{Shift}1 >> MoveToDesktop(1)
```

***

## 开发计划

### Phase 1: Windows 基础架构 ✅ 已完成

#### 1.1 项目搭建 ✅

* [x] 创建 Cargo 项目结构（workspace: wakem, wakemd, wakemctl）
* [x] 配置依赖（windows-rs, serde, anyhow, parking\_lot 等）
* [ ] 设置代码规范（rustfmt, clippy）
* [ ] 创建 GitHub Actions CI/CD

**项目结构**:
```
crates/
├── wakem-common/    # 共享组件（类型、配置、IPC）
├── wakemd/          # 服务端 daemon
├── wakem/           # 客户端 GUI
└── wakemctl/        # 命令行控制工具
```

#### 1.2 核心数据结构 ✅

* [x] 定义输入事件结构（KeyEvent, MouseEvent）
* [x] 定义输出动作结构（KeyAction, MouseAction, WindowAction）
* [x] 定义映射规则结构（MappingRule, ContextCondition）
* [x] 实现按键状态机（KeyState, ModifierState）

**关键文件**:
- `crates/wakem-common/src/types/input.rs` - 输入事件类型
- `crates/wakem-common/src/types/action.rs` - 动作类型
- `crates/wakem-common/src/types/mapping.rs` - 映射规则

#### 1.3 进程通信（IPC）✅

* [x] 设计通信协议（Message 枚举，序列化/反序列化）
* [x] 实现 Named Pipe 服务端（wakemd）
* [x] 实现 Named Pipe 客户端（wakem）
* [x] 实现命令行控制工具（wakemctl）
* [ ] 添加连接重试和错误处理

**关键文件**:
- `crates/wakem-common/src/ipc/mod.rs` - 通信协议
- `crates/wakem-common/src/ipc/server.rs` - IPC 服务端
- `crates/wakem-common/src/ipc/client.rs` - IPC 客户端

#### 1.4 配置系统 ✅

* [x] 设计配置格式（参考 keymapper.conf）
* [x] 实现配置解析器（TOML 格式）
* [ ] 实现配置验证（语义检查、错误提示）
* [ ] 实现配置热重载（文件监听、动态更新）
* [x] 编写配置文档和示例

**关键文件**:
- `crates/wakem-common/src/config/mod.rs` - 配置解析
- `wakem.conf` - 示例配置文件

#### 1.5 Windows 输入抓取 ✅

* [x] 注册 Raw Input 设备（键盘、鼠标）
* [x] 实现消息循环和事件分发
* [x] 处理按键扫描码和虚拟键码转换
* [ ] 实现输入事件队列（线程安全）
* [x] 添加调试日志和事件追踪

**关键文件**:
- `crates/wakemd/src/platform/windows/input.rs` - Raw Input 实现

#### 1.6 Windows 输入发送 ✅

* [x] 实现 SendInput 包装（键盘事件）
* [x] 实现 SendInput 包装（鼠标事件）
* [x] 处理修饰键状态同步
* [ ] 实现输入序列的批量发送
* [ ] 添加发送延迟和速率控制

**关键文件**:
- `crates/wakemd/src/platform/windows/output.rs` - SendInput 实现

#### 1.7 基础映射引擎 ✅

* [x] 实现基础映射表（扫描码 -> 动作）
* [x] 实现映射查找和匹配逻辑
* [x] 处理单键重映射（如 CapsLock -> Backspace）
* [ ] 处理修饰键交换（如 LAlt <-> LCtrl）
* [ ] 添加映射冲突检测

**关键文件**:
- `crates/wakemd/src/runtime/mapper.rs` - 映射引擎

***

### Phase 2: Windows 键盘增强 ✅ 已完成

#### 2.1 键位重映射基础 ✅

* [x] 实现基础映射表（扫描码 -> 扫描码）

* [x] 实现映射查找和匹配逻辑

* [x] 处理单键重映射（如 CapsLock -> Backspace）

* [ ] 处理修饰键交换（如 LAlt <-> LCtrl）

* [ ] 添加映射冲突检测

**关键文件**:
- `crates/wakemd/src/runtime/mapper.rs` - 映射引擎
- `crates/wakem-common/src/config/mod.rs` - 配置解析

#### 2.2 快捷键层系统 ✅

* [x] 设计层数据结构（Layer, LayerStack）

* [x] 实现层切换逻辑（按住触发、切换触发）

* [x] 实现层内映射查找

* [x] 处理多层叠加和优先级

* [ ] 添加层状态指示（可选：托盘图标变化）

**关键文件**:
- `crates/wakem-common/src/types/layer.rs` - 层类型定义
- `crates/wakemd/src/runtime/layer_manager.rs` - 层管理器

#### 2.3 导航层配置 ✅

以下功能通过配置文件实现：

```toml
[keyboard.layers.navigate]
activation_key = "CapsLock"
mode = "Hold"

[keyboard.layers.navigate.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"
I = "Home"
O = "End"
U = "PageUp"
P = "PageDown"
```

* [x] 验证导航层配置可正常解析和生效

* [ ] 添加层切换的视觉反馈（可选）

#### 2.4 上下文感知 ✅

* [x] 实现前台窗口检测（GetForegroundWindow）

* [x] 实现窗口类名/进程名获取

* [x] 实现上下文匹配逻辑（通配符）

* [ ] 实现上下文切换的映射更新

* [ ] 添加上下文调试信息

**关键文件**:
- `crates/wakemd/src/platform/windows/context.rs` - 上下文检测

#### 2.5 快速启动 ✅

* [x] 实现 Launch 动作类型

* [x] 实现进程启动

* [x] 支持带参数的程序启动

* [ ] 支持工作目录设置

* [x] 添加启动错误处理

**关键文件**:
- `crates/wakemd/src/platform/windows/launcher.rs` - 程序启动器

#### 2.6 高级键盘功能（待实现）

* [ ] 实现文本扩展（缩写展开）

* [ ] 实现宏录制和回放

* [ ] 实现条件触发（双击、长按）

* [ ] 实现按键序列（chord）

***

### Phase 3: Windows 窗口管理

#### 3.1 窗口信息获取

* [ ] 封装窗口枚举（EnumWindows）

* [ ] 获取窗口标题、类名、进程信息

* [ ] 获取窗口位置和大小

* [ ] 获取窗口状态（最小化、最大化、置顶）

* [ ] 实现窗口信息缓存

#### 3.2 窗口操作基础

* [ ] 实现窗口移动（SetWindowPos）

* [ ] 实现窗口调整大小

* [ ] 实现窗口最小化/最大化/还原

* [ ] 实现窗口关闭（WM\_CLOSE）

* [ ] 实现窗口置顶切换

#### 3.3 窗口位置预设

* [ ] 计算屏幕区域（单显示器、多显示器）

* [ ] 实现左半屏、右半屏、上半屏、下半屏

* [ ] 实现四角定位（左上、右上、左下、右下）

* [ ] 实现居中、最大化、1/3屏、1/4屏

* [ ] 支持多显示器间的窗口移动

#### 3.4 窗口切换基础

* [ ] 实现窗口列表获取（按进程分组）

* [ ] 实现 Z-Order 排序

* [ ] 实现窗口切换逻辑（SetForegroundWindow）

* [ ] 处理最小化窗口的还原

* [ ] 实现 Alt+\` 同进程窗口切换

#### 3.5 窗口切换 UI

* [ ] 创建分层窗口（WS\_EX\_LAYERED）

* [ ] 实现 GDI+ 初始化和资源管理

* [ ] 绘制背景（圆角矩形、主题色）

* [ ] 绘制应用图标（HICON 渲染）

* [ ] 实现选中高亮效果

* [ ] 实现窗口位置计算（居中显示）

#### 3.6 图标系统

* [ ] 实现图标获取（从 EXE、从窗口）

* [ ] 实现 UWP 应用图标解析（AppxManifest.xml）

* [ ] 实现图标缓存（避免重复加载）

* [ ] 实现图标覆盖配置

* [ ] 处理图标缩放和裁剪

#### 3.7 虚拟桌面支持

* [ ] 检测虚拟桌面 API 可用性

* [ ] 实现当前桌面窗口过滤（DWM cloak 检测）

* [ ] 实现虚拟桌面切换（COM 接口）

* [ ] 实现窗口移动到指定桌面

***

### Phase 4: Windows 鼠标增强

#### 4.1 鼠标事件处理

* [ ] 扩展 Raw Input 支持鼠标

* [ ] 处理鼠标移动、点击、滚轮事件

* [ ] 实现鼠标按钮重映射

* [ ] 实现鼠标滚轮重映射

#### 4.2 鼠标手势

* [ ] 设计手势识别算法（方向、距离、速度）

* [ ] 实现手势状态机（按下 -> 移动 -> 释放）

* [ ] 支持基本手势（上、下、左、右、圆）

* [ ] 实现手势到动作的绑定

* [ ] 添加手势轨迹显示（可选）

#### 4.3 滚轮增强

* [ ] 实现滚轮事件拦截和转换

* [ ] 支持在特定区域调节音量/亮度

* [ ] 支持在标签栏切换标签

* [ ] 实现滚轮加速（快速滚动）

#### 4.4 边缘触发

* [ ] 实现屏幕边缘检测

* [ ] 实现触发延迟和防抖动

* [ ] 支持角落触发

* [ ] 绑定边缘触发动作

***

### Phase 5: Windows 完善

#### 5.1 系统托盘

* [ ] 创建托盘图标（NOTIFYICONDATA）

* [ ] 实现右键菜单（启用/禁用、配置、退出）

* [ ] 实现启动项管理（注册表/计划任务）

* [ ] 处理托盘图标重建（任务栏重启）

#### 5.2 配置热重载

* [ ] 实现文件系统监听（ReadDirectoryChangesW）

* [ ] 实现配置重新加载

* [ ] 处理配置错误（回滚到上次有效配置）

* [ ] 添加配置变更通知（托盘提示）

#### 5.3 错误处理和日志

* [ ] 实现日志系统（分级、文件/控制台输出）

* [ ] 添加错误报告和崩溃处理

* [ ] 实现日志轮转

* [ ] 添加性能监控（可选）

#### 5.4 安装和打包

* [ ] 创建 MSI 安装程序

* [ ] 实现自动更新检查

* [ ] 编写用户文档

* [ ] 编写开发文档

***

### Phase 6: macOS 移植

#### 6.1 macOS 基础架构

* [ ] 设置 macOS 开发环境

* [ ] 实现 IOKit 输入抓取

* [ ] 实现 CGEventPost 输入发送

* [ ] 实现 Unix Socket IPC

#### 6.2 macOS 窗口管理

* [ ] 实现 Carbon/Accessibility API 窗口操作

* [ ] 适配窗口切换 UI（Core Graphics）

* [ ] 实现 Mission Control 集成

#### 6.3 macOS 系统集成

* [ ] 实现 Cocoa 系统托盘

* [ ] 实现启动项管理（LaunchAgent）

* [ ] 适配 macOS 主题

***

### Phase 7: Linux 移植

#### 7.1 Linux 基础架构

* [ ] 设置 Linux 开发环境

* [ ] 实现 evdev 输入抓取

* [ ] 实现 uinput 输入发送

* [ ] 实现 Unix Socket IPC

#### 7.2 Linux 窗口管理（Wayland）

* [ ] 调研 Wayland 协议限制和安全模型

* [ ] 实现 Wayland 客户端（wlroots 扩展或 compositor 特定协议）

* [ ] 实现窗口枚举和焦点管理（通过 compositor 扩展）

* [ ] 适配窗口切换 UI（Cairo/OpenGL）

* [ ] 适配 GNOME/Mutter 的 Wayland 扩展协议

#### 7.3 Linux 系统集成

* [ ] 实现 GTK/AppIndicator 系统托盘

* [ ] 实现 systemd 用户服务

* [ ] 适配桌面环境主题

***

## 参考项目

| 项目                                                            | 语言   | 核心特点           | 学习重点              | 本地路径                      |
| ------------------------------------------------------------- | ---- | -------------- | ----------------- | ------------------------- |
| [keymapper](https://github.com/houmain/keymapper)             | C++  | 跨平台、客户端-服务端架构  | 架构设计、配置语法、输入处理    | `keymapper-5.5.0/`        |
| [AutoHotkey](https://github.com/AutoHotkey/AutoHotkey)        | C++  | 完整脚本语言、强大热键系统  | 热键变体、窗口操作、消息循环    | `AutoHotkey-2.0.23/`      |
| [window-switcher](https://github.com/sigoden/window-switcher) | Rust | 精致窗口切换、GDI+ 界面 | 窗口切换 UI、图标获取、虚拟桌面 | `window-switcher-1.17.0/` |

### 本地参考项目结构

```
wakem/
├── AutoHotkey-2.0.23/          # AutoHotkey 源码
│   └── source/
│       ├── ahklib.cpp          # COM 接口实现，脚本执行入口
│       ├── application.cpp     # 应用程序主逻辑
│       ├── window.cpp          # 窗口操作封装
│       ├── keyboard.cpp        # 键盘输入处理
│       └── ...
│
├── keymapper-5.5.0/            # keymapper 源码
│   ├── src/
│   │   ├── client/             # 客户端实现
│   │   │   └── windows/
│   │   │       └── main.cpp    # Windows 客户端入口，系统托盘
│   │   ├── server/             # 服务端实现
│   │   ├── config/             # 配置解析
│   │   └── runtime/            # 运行时核心
│   └── ...
│
└── window-switcher-1.17.0/     # window-switcher 源码
    └── src/
        ├── app.rs               # 应用程序主逻辑，窗口管理
        ├── painter.rs           # GDI+ 绘制实现
        ├── trayicon.rs          # 系统托盘
        ├── keyboard.rs          # 键盘监听
        └── ...
```

### 关键参考代码

#### 1. AutoHotkey - 热键系统和窗口操作

* `source/ahklib.cpp` - DLL 接口和脚本执行

* `source/window.cpp` - Win32 API 窗口操作封装

* `source/keyboard.cpp` - 底层键盘输入处理

* `source/script.cpp` - 脚本解析和执行

#### 2. keymapper - 客户端-服务端架构

* `src/client/windows/main.cpp` - Windows 客户端实现

  * 系统托盘图标管理

  * 与服务端的 IPC 通信

  * 配置热重载

  * 上下文感知（前台窗口检测）

* `src/server/windows/` - Windows 服务端实现

  * Raw Input 输入抓取

  * SendInput 输入发送

  * 权限处理

#### 3. window-switcher - Rust Windows 编程

* `src/app.rs` - 应用程序主逻辑

  * 窗口消息循环

  * 应用切换状态管理

  * 图标缓存

* `src/painter.rs` - GDI+ 绘制

  * 分层窗口创建

  * 图标渲染

  * 圆角矩形背景

* `src/keyboard.rs` - 低级别键盘钩子

* `src/trayicon.rs` - 系统托盘实现

***

## 文档索引

* [Keymapper 完整指南](./docs/keymapper-complete-guide.md) - keymapper 项目详细分析

* [AutoHotkey 项目分析](./docs/autohotkey-analysis.md) - AutoHotkey 架构分析

* [Window Switcher 分析](./docs/window-switcher-analysis.md) - window-switcher 实现细节

