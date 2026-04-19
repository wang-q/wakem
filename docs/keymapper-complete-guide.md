# Keymapper 完整指南

## 目录

1. [项目概述](#项目概述)
2. [项目架构](#项目架构)
3. [系统架构详解](#系统架构详解)
4. [配置指南](#配置指南)
5. [工作流程](#工作流程)
6. [关键技术点](#关键技术点)

---

## 项目概述

**Keymapper** 是一个跨平台的上下文感知键盘重映射工具，支持 GNU/Linux、Windows、macOS 和 FreeBSD 系统。它允许用户通过单一配置文件管理系统级或应用级的键盘快捷键和布局重映射。

### 核心功能

1. **键盘布局重定义** - 系统级或应用级键盘布局自定义
2. **快捷键管理** - 在单一配置文件中管理所有快捷键
3. **上下文感知** - 根据当前窗口标题、类名、进程路径等动态切换映射
4. **鼠标支持** - 支持鼠标按钮和滚轮映射
5. **字符输入** - 可直接指定字符而非按键进行映射
6. **应用启动** - 绑定快捷键启动应用程序
7. **虚拟键状态** - 支持状态切换和层切换

---

## 项目架构

### 目录结构

```
keymapper-5.5.0/
├── src/
│   ├── client/          # 客户端组件（配置管理、UI、系统交互）
│   ├── server/          # 服务端组件（设备抓取、按键转发）
│   ├── config/          # 配置解析
│   ├── runtime/         # 运行时核心（按键匹配、状态管理）
│   ├── control/         # 控制工具（keymapperctl）
│   ├── common/          # 共享组件
│   ├── test/            # 测试代码
│   └── libs/            # 第三方库
├── extra/               # 额外资源（图标、启动脚本等）
├── cmake/               # CMake 模块
├── README.md
├── CHANGELOG.md
├── LICENSE
└── keymapper.conf       # 示例配置文件
```

### 核心组件

#### 1. Client（客户端）- keymapper

负责配置管理、窗口焦点跟踪、系统托盘、与控制端通信。

**关键文件：**
- `client/main.cpp` - 客户端入口
- `client/ClientState.h/cpp` - 客户端状态管理
- `client/ConfigFile.cpp` - 配置文件处理
- `client/FocusedWindow.h/cpp` - 窗口焦点检测（X11/Wayland/Carbon/Win32）
- `client/ServerPort.cpp` - 与服务端通信
- `client/ControlPort.cpp` - 处理控制端命令

**平台特定实现：**
- `client/windows/` - Windows 实现
- `client/unix/` - Linux/macOS 实现（通过条件编译和运行时检测共存）

  虽然 macOS 和 Linux 的窗口系统差异很大，但它们通过以下方式在 `client/unix/` 目录下共存：

  1. **条件编译**：使用宏如 `ENABLE_X11`、`ENABLE_CARBON`、`ENABLE_WAYLAND` 等控制编译
  2. **运行时检测**：在 `FocusedWindowImpl.cpp` 和 `StringTyperImpl.cpp` 中，代码会尝试初始化多个系统，哪个成功就使用哪个
  3. **分离的实现文件**：
     - Linux: `FocusedWindowX11.cpp`, `FocusedWindowWlroots.cpp`, `FocusedWindowDBus.cpp`, `StringTyperX11.cpp`, `StringTyperWayland.cpp`, `StringTyperXKB.cpp`
     - macOS: `FocusedWindowCarbon.cpp`, `StringTyperCarbon.cpp`, `TrayIconCocoa.mm`
  4. **通用回退**：`StringTyperGeneric.cpp` 提供通用实现作为后备

  例如窗口检测的初始化逻辑：
  ```cpp
  const auto systems = std::initializer_list<std::pair<const char*, MakeFocusedWindowSystem*>>{
  #if defined(ENABLE_X11)
      { "X11", &make_focused_window_x11 },
  #endif
  #if defined(ENABLE_CARBON)
      { "Carbon", &make_focused_window_carbon },
  #endif
      // ...
  };
  // 遍历尝试初始化，成功的加入系统列表
  ```

#### 2. Server（服务端）- keymapperd

负责底层设备抓取、虚拟设备创建、按键事件转发。

**关键文件：**
- `server/main.cpp` - 服务端入口
- `server/ServerState.h/cpp` - 服务端状态管理
- `server/ClientPort.cpp` - 与客户端通信

**平台特定实现：**
- `server/windows/Devices.cpp` - Windows 设备管理
- `server/unix/GrabbedDevicesLinux.cpp` - Linux 设备抓取
- `server/unix/VirtualDevicesLinux.cpp` - Linux 虚拟设备
- `server/unix/GrabbedDevicesMacOS.cpp` - macOS 设备抓取

#### 3. Config（配置解析）

负责解析 keymapper.conf 配置文件。

**关键文件：**
- `config/ParseConfig.cpp` - 主配置解析器
- `config/ParseKeySequence.cpp` - 按键序列解析
- `config/Config.h` - 配置数据结构定义
- `config/get_key_name.cpp` - 键名处理

#### 4. Runtime（运行时核心）

负责按键匹配、状态转换、多阶段处理。

**关键文件：**
- `runtime/Stage.cpp` - 单阶段处理
- `runtime/MultiStage.cpp` - 多阶段协调
- `runtime/MatchKeySequence.cpp` - 按键序列匹配
- `runtime/Key.h` - 键码定义
- `runtime/KeyEvent.h` - 按键事件定义

#### 5. Control（控制工具）- keymapperctl

命令行工具，用于外部控制 keymapper。

**关键文件：**
- `control/main.cpp` - 控制工具入口
- `control/ClientPort.cpp` - 与客户端通信

---

## 系统架构详解

### 系统架构图

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

### 模块职责

#### Client 模块 (keymapper)

**职责：**
- 配置文件加载和监控
- 窗口焦点检测
- 系统托盘/通知
- 与服务端通信
- 处理控制命令

**核心类：**

```cpp
class ClientState {
    ConfigFile m_config_file;        // 配置文件管理
    ServerPort m_server;             // 服务端通信端口
    ControlPort m_control;           // 控制端口
    FocusedWindow m_focused_window;  // 窗口焦点跟踪
    std::vector<int> m_active_contexts; // 活动上下文
};
```

**平台实现：**

| 功能 | Windows | Linux | macOS |
|------|---------|-------|-------|
| 窗口检测 | Win32 API | X11/Wayland/DBus | Carbon |
| 字符串输入 | SendInput | XTest/xkbcommon | Carbon |
| 系统托盘 | Win32 | AppIndicator | Cocoa |

#### Server 模块 (keymapperd)

**职责：**
- 输入设备抓取
- 虚拟设备创建
- 按键事件处理
- 按键转发

**核心类：**

```cpp
class ServerState {
    std::unique_ptr<IClientPort> m_client;  // 客户端通信
    std::unique_ptr<MultiStage> m_stage;    // 多阶段处理器
    std::vector<KeyEvent> m_send_buffer;    // 发送缓冲区
    std::vector<Key> m_virtual_keys_down;   // 虚拟键状态
};
```

**平台实现：**

| 功能 | Windows | Linux | macOS |
|------|---------|-------|-------|
| 设备抓取 | Raw Input / Interception | evdev | Karabiner-DriverKit |
| 虚拟设备 | SendInput | uinput | Karabiner VirtualHID |

#### Config 模块

**职责：**
- 配置文件解析
- 按键序列解析
- 配置数据结构

**核心数据结构：**

```cpp
struct Config {
    struct Context {
        Filter window_class_filter;     // 窗口类过滤器
        Filter window_title_filter;     // 窗口标题过滤器
        Filter window_path_filter;      // 进程路径过滤器
        Filter device_filter;           // 设备过滤器
        KeySequence modifier_filter;    // 修饰键过滤器
        std::vector<Input> inputs;      // 输入映射
        std::vector<KeySequence> outputs; // 输出序列
    };
    std::vector<Context> contexts;      // 所有上下文
};
```

#### Runtime 模块

**职责：**
- 按键序列匹配
- 状态管理
- 多阶段处理

**核心类：**

```cpp
class Stage {
    // 单阶段按键处理
    bool advance(KeyEvent event, int* output_index);
    bool is_clear() const;
};

class MultiStage {
    // 多阶段协调
    std::vector<StagePtr> m_stages;
    bool advance(KeyEvent event, KeySequence& output);
};
```

#### Control 模块 (keymapperctl)

**职责：**
- 命令行控制接口
- 按键注入
- 虚拟键控制

**命令：**
```bash
keymapperctl --input <sequence>      # 注入输入序列
keymapperctl --output <sequence>     # 注入输出序列
keymapperctl --type "string"         # 输入字符串
keymapperctl --press <key>           # 按下按键
keymapperctl --release <key>         # 释放按键
keymapperctl --toggle <key>          # 切换虚拟键
```

### 数据流

#### 按键处理流程

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│  Device  │───>│  Server  │───>│  Stage   │───>│  Output  │
│  Event   │    │  Grab    │    │  Match   │    │  Device  │
└──────────┘    └──────────┘    └──────────┘    └──────────┘
                                     │
                                     v
                              ┌──────────┐
                              │  Config  │
                              │  Context │
                              └──────────┘
```

#### 上下文切换流程

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Window      │───>│  Client      │───>│  Server      │
│  Focus       │    │  Detect      │    │  Update      │
│  Change      │    │  Context     │    │  Mappings    │
└──────────────┘    └──────────────┘    └──────────────┘
```

### 关键算法

#### 按键序列匹配

```cpp
// 伪代码
bool match(KeySequence input, KeySequence pattern) {
    // 1. 检查前缀匹配
    if (!starts_with(input, pattern.prefix))
        return false;
    
    // 2. 检查剩余部分
    for (auto& event : input.remaining) {
        if (!can_match(event, pattern))
            return false;
    }
    
    // 3. 检查超时
    if (pattern.has_timeout) {
        if (elapsed > pattern.timeout)
            return false;
    }
    
    return true;
}
```

#### 多阶段处理

```cpp
// 伪代码
KeySequence process(KeySequence input) {
    KeySequence current = input;
    
    for (auto& stage : stages) {
        KeySequence output;
        for (auto& event : current) {
            if (auto mapped = stage.map(event))
                output.append(mapped);
            else
                output.append(event);
        }
        current = output;
    }
    
    return current;
}
```

### 平台抽象层

#### 设备接口

```cpp
class IDeviceGrabber {
public:
    virtual bool grab() = 0;
    virtual bool ungrab() = 0;
    virtual KeyEvent read_event() = 0;
    virtual bool write_event(KeyEvent event) = 0;
};
```

#### 窗口检测接口

```cpp
class IFocusedWindow {
public:
    virtual WindowInfo get_focused_window() = 0;
    virtual bool initialize() = 0;
};
```

### 配置加载流程

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Read File  │───>│  Parse      │───>│  Build      │───>│  Validate   │
│             │    │  Config     │    │  Contexts   │    │  & Send     │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
                          │
                          v
                   ┌─────────────┐
                   │  Parse      │
                   │  KeySeq     │
                   └─────────────┘
```

### 性能考虑

1. **按键延迟** - 直接转发未匹配的按键，最小化处理
2. **匹配优化** - 使用前缀树加速模式匹配
3. **缓冲区管理** - 预分配发送缓冲区，减少内存分配
4. **事件批处理** - 批量发送按键事件

### 安全考虑

1. **紧急退出** - Shift+Escape+K 终止服务端
2. **权限控制** - 服务端需要 root/管理员权限
3. **配置验证** - 启动时验证配置语法
4. **设备过滤** - 可选择性抓取设备

---

## 配置指南

### 配置文件位置

keymapper 按以下顺序查找配置文件 `keymapper.conf`：

1. `$XDG_CONFIG_HOME/keymapper/keymapper.conf`
2. `$HOME/.config/keymapper/keymapper.conf`
3. Linux/macOS/FreeBSD: `/etc/keymapper.conf`
4. Windows: `%USERPROFILE%\keymapper.conf`
5. Windows: `%LOCALAPPDATA%\keymapper\keymapper.conf`
6. Windows: `%APPDATA%\keymapper\keymapper.conf`
7. 当前工作目录

也可通过 `-c` 参数指定配置文件路径。

### 基本语法

#### 注释

```bash
# 这是注释
```

#### 基本映射

```bash
<input> >> <output>
```

示例：
```bash
CapsLock >> Backspace
Z >> Y
Y >> Z
Control{Q} >> Alt{F4}
```

### 输入表达式

#### 按键组合

| 语法 | 说明 | 示例 |
|------|------|------|
| `A B` | 依次按下 | `A B >> C` |
| `(A B)` | 同时按下 | `(Shift A) >> B` |
| `A{B}` | 按住A时按B | `Control{C} >> Copy` |
| `A{B{C}}` | 嵌套 | `Shift{Control{A}} >> B` |
| `(A B){C}` | 组合修饰 | `(Shift Control){A} >> B` |

#### 特殊前缀

| 前缀 | 说明 | 示例 |
|------|------|------|
| `!` | 未按下/释放 | `!A >> B` |
| `?` | 部分匹配跳过 | `? "abc" >> matched` |

#### 超时

```bash
# 按住CapsLock 500ms触发Escape
CapsLock{500ms} >> Escape

# 快速按下并释放Control触发Escape
Control{!250ms} >> Escape

# A后250ms内按B触发C
A !250ms B >> C

# 双击A（200ms内）触发D
A{!200ms} !200ms A{!200ms} >> D
```

### 输出表达式

#### 按键输出

```bash
# 依次输出
A >> B C D

# 同时输出
A >> (B C)

# 按住输出
A >> Shift{B}

# 释放按键
A >> !Shift B
```

#### 按下/释放分离

```bash
# ^ 分隔按下和释放时的输出

# 按下输出B，释放输出C
A >> B ^ C

# 仅释放时输出
A >> ^B

# 仅按下时输出（阻止重复）
A >> B^
```

#### 字符串输出

```bash
# 直接输出字符
AltRight{A} >> '@'

# 多行字符串（使用\续行）
Meta{A} K >> \
  "Kind regards,\n" \
  "Douglas Quaid"
```

#### 命令执行

```bash
# Windows
Meta{C} >> $(start cmd) ^
Meta{W} >> $(C:\windows\system32\calc.exe) ^

# Linux
Meta{W} >> $(exo-open --launch WebBrowser) ^

# 使用^防止重复执行
```

### 上下文块

#### 基本语法

```ini
[default]
# 默认上下文

[system = "Linux"]
# 仅Linux系统

[title = "Visual Studio Code"]
# 窗口标题包含"Visual Studio Code"

[class = "qtcreator"]
# 窗口类为"qtcreator"

[path = "notepad.exe"]
# 进程路径包含"notepad.exe"

[device = "Device Name"]
# 特定输入设备
```

#### 条件组合

```ini
# 多个条件（AND）
[system = "Linux", class = "qtcreator"]

# 否定条件
[class != "qtcreator"]

# 正则表达式（不区分大小写）
[title = /Visual Studio Code|Code OSS/i]
```

#### 修饰键上下文

```ini
# Virtual1按下且Virtual2未按下时激活
[modifier = "Virtual1 !Virtual2"]
```

#### 字符串比较

```ini
# 根据环境变量激活
[getenv["HOSTNAME"] = "LaptopMum"]
```

### 虚拟键

#### 定义和使用

```bash
# 定义虚拟键
VimMode = Virtual1
CapsWord = Virtual2

# 切换虚拟键
ScrollLock >> VimMode

# 使用虚拟键作为修饰符
VimMode{A} >> ArrowLeft
VimMode{S} >> ArrowDown

# 条件映射（虚拟键未按下时）
!VimMode A >> B

# 释放时触发
!VimMode >> Escape
```

#### ContextActive

```ini
# 进入/离开上下文时触发
[title = "Firefox"]
ContextActive >> Virtual1 ^ !Virtual1
```

### 别名和宏

#### 简单别名

```bash
# 键别名
Win = Meta
Alt = AltLeft | AltRight

# 序列别名
proceed = Tab Tab Enter
```

#### 参数化宏

```bash
# 定义宏
swap = $0 >> $1; $1 >> $0

# 使用宏
swap[Y, Z]
```

#### 字符串插值

```bash
greet = "Hello"
F1 >> "${greet} World"  # 输出 "Hello World"
```

#### 内置宏

| 宏 | 说明 | 示例 |
|----|------|------|
| `repeat[EXPR, N]` | 重复 | `repeat[Backspace, 3]` |
| `length[STR]` | 字符串长度 | `length["abc"]` → 3 |
| `default[A, B]` | 默认值 | `default[$0, "default"]` |
| `apply[EXPR, ARGS...]` | 批量应用 | `apply[F$0 >> Meta{$0}, 1, 2, 3]` |
| `add[A, B]` | 加法 | `add[1, 2]` → 3 |
| `sub[A, B]` | 减法 | `sub[5, 3]` → 2 |
| `getenv["VAR"]` | 环境变量 | `getenv["HOME"]` |

#### 高级宏示例

```bash
# 文本替换
substitute = ? "$0" >> repeat[Backspace, sub[length["$0"], 1]] "$1"
substitute["Cat", "Dog"]

# 批量生成映射
apply[F$0 >> Meta{$0}, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
# 生成 F1 >> Meta{1}, F2 >> Meta{2}, ...
```

### 指令

#### @forward-modifiers

```bash
# 立即转发修饰键（推荐添加）
@forward-modifiers Shift Control Alt
```

#### @include

```bash
# 包含其他配置文件
@include "common.conf"

# 可选包含（文件不存在不报错）
@include-optional "$HOME/.keymapper_local.conf"
```

#### @options

```bash
# 设置命令行选项
@options update no-tray no-notify verbose
```

#### @toggle-active

```bash
# 设置切换keymapper激活状态的快捷键
@toggle-active ScrollLock
```

#### @virtual-keys-toggle

```bash
# 设置虚拟键行为（默认true）
@virtual-keys-toggle true   # 切换模式
@virtual-keys-toggle false  # 显式按下/释放模式
```

#### @grab-device / @skip-device

```bash
# 只抓取特定设备
@skip-device /.*/
@grab-device "My Keyboard"
```

#### @done

```bash
# 停止解析配置文件
@done
```

#### 平台特定指令

```bash
# Linux
@linux-compose-key AltRight{Insert}
@linux-highres-wheel-events

# macOS
@macos-toggle-fn
@macos-iso-keyboard
```

### 多阶段

```ini
# 阶段1：布局调整
Z >> Y
Y >> Z

[stage]

# 阶段2：快捷键映射（接收阶段1的输出）
Control{Z} >> undo
```

### 抽象命令

```bash
# 定义抽象命令
Control{B} >> build

# 在不同上下文映射到不同输出
build >> Control{B}

[title="Visual Studio"]
build >> (Shift Control){B}
```

### 常用模式

#### CapsLock 改 Backspace

```bash
CapsLock >> Backspace
Control{CapsLock} >> CapsLock
```

#### 方向键层

```bash
Ext = IntlBackslash

Ext{I} >> ArrowUp
Ext{K} >> ArrowDown
Ext{J} >> ArrowLeft
Ext{L} >> ArrowRight
```

#### 应用特定排除

```ini
# 在远程桌面中禁用所有映射
[title="Remote Desktop"]
Any >> Any

[default]
```

#### 快速输入模板

```bash
signature = "Best regards,\nJohn Doe"
Meta{S} >> signature
```

#### 窗口管理

```bash
Win{Q} >> close_window
Win{A} >> lower_window
```

### 调试技巧

#### 查看按键信息

```bash
# 使用托盘图标"Next Key Info"功能
# 或使用命令行
keymapperctl --next-key-info
```

#### 详细日志

```bash
# 启动时添加verbose选项
keymapper -v
# 或在配置中
@options verbose
```

#### 紧急退出

```
Shift + Escape + K
```

### 完整配置示例

```bash
# ===== 基础设置 =====
@forward-modifiers Shift Control Alt

# ===== 别名定义 =====
Alt     = AltLeft
AltGr   = AltRight
Win     = Meta
Command = Meta

# ===== 基础映射 =====
CapsLock >> Backspace
Control{CapsLock} >> CapsLock

# ===== 扩展层 =====
Ext = IntlBackslash

# 方向键
Ext{I} >> ArrowUp
Ext{K} >> ArrowDown
Ext{J} >> ArrowLeft
Ext{L} >> ArrowRight

# 功能键
Ext{U} >> Home
Ext{O} >> End
Ext{Y} >> PageUp
Ext{H} >> PageDown

# 编辑操作
Ext{X} >> Control{X}
Ext{C} >> Control{C}
Ext{V} >> Control{V}
Ext{Z} >> Control{Z}

# ===== 窗口管理 =====
Win{Q} >> close_window
Win{A} >> lower_window

# ===== 应用特定 =====
[title="Visual Studio Code"]
Ext{G} >> F12  # 跳转到定义

[system="Linux" class="tilix"]
close_window >> (Shift Control){W}

[system="Windows" class="CabinetWClass"]
open_terminal >> F4 ^ Control{A} "cmd" Enter
```

---

## 工作流程

### 1. 启动流程

1. **keymapper** 启动，加载配置文件
2. 连接到 **keymapperd** 服务端
3. 发送配置到服务端
4. 检测当前窗口焦点，发送活动上下文

### 2. 按键处理流程

1. **keymapperd** 从物理设备抓取按键事件
2. 按键事件追加到按键序列
3. 按键序列与输入表达式匹配
4. 匹配成功则输出映射的按键序列
5. 未匹配则转发原始按键

### 3. 上下文切换流程

1. **keymapper** 检测窗口焦点变化
2. 计算匹配的上下文
3. 发送活动上下文索引到服务端
4. 服务端更新活动映射

---

## 关键技术点

### 跨平台实现

| 平台 | 设备抓取 | 虚拟设备 | 窗口检测 |
|------|----------|----------|----------|
| Linux | evdev (uinput) | uinput | X11/Wayland/DBus |
| Windows | Raw Input/Interception | SendInput | Win32 API |
| macOS | Karabiner-DriverKit | Karabiner VirtualHID | Carbon/Cocoa |

### 按键匹配算法

- 按键序列按顺序匹配
- 支持部分匹配和回溯
- 最长匹配优先
- 支持超时和虚拟键状态

### 多阶段处理

```ini
# 阶段 1：布局调整
Z >> Y
Y >> Z

[stage]

# 阶段 2：快捷键映射
Control{Z} >> undo
```

---

## 代码统计

- **总文件数**: ~80 个源文件
- **核心模块**: client, server, config, runtime, control, common
- **测试覆盖**: test0-5 覆盖解析、匹配、阶段、服务端、模糊测试
- **平台代码**: Windows (~10 文件), Unix (~20 文件)

---

## 许可证

GNU General Public License v3.0

---

## 版本信息

- **当前版本**: 5.5.0 (2026-04-17)
- **主要更新**: 新增 macOS 鼠标支持、增加动作限制到 1024、添加 `keymapperctl --type-stdin`

---

## 参考链接

- 项目主页: https://github.com/houmain/keymapper
- 配置示例: [keymapper.conf](../keymapper-5.5.0/keymapper.conf)
- 完整文档: [README.md](../keymapper-5.5.0/README.md)
