# wakem 配置指南

本文档包含 wakem 的完整配置说明。

## 配置文件位置

### Windows

wakem 使用以下目录结构（遵循 XDG Base Directory 规范的 Windows 适配）：

| 类型 | 路径 | 说明 |
|------|------|------|
| 程序 | `%LOCALAPPDATA%\Programs\wakem\` | 可执行文件安装位置 |
| 配置 | `%APPDATA%\wakem\` | 配置文件目录 |
| 数据 | `%LOCALAPPDATA%\wakem\` | 日志等数据文件 |
| 启动项 | `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\` | 开机启动快捷方式 |

配置文件按以下优先级查找（找到即停止）：

| 优先级 | 路径（实例 0） | 路径（实例 N） | 说明 |
|:------:|----------------|----------------|------|
| 1 | `%USERPROFILE%\.wakem.toml` | `%USERPROFILE%\.wakem-instanceN.toml` | 用户主目录下的单文件配置 |
| 2 | `%APPDATA%\wakem\config.toml` | `%APPDATA%\wakem\config-instanceN.toml` | 配置目录（推荐） |

**推荐**：使用 `%APPDATA%\wakem\config.toml`，遵循 Windows 标准配置目录规范。

> `%APPDATA%` 通常指向 `C:\Users\<用户名>\AppData\Roaming`，`%LOCALAPPDATA%` 通常指向 `C:\Users\<用户名>\AppData\Local`

### macOS

| 优先级 | 路径（实例 0） | 路径（实例 N） | 说明 |
|:------:|----------------|----------------|------|
| 1 | `~/.wakem.toml` | `~/.wakem-instanceN.toml` | 用户主目录（向后兼容） |
| 2 | `~/.config/wakem/config.toml` | `~/.config/wakem/config-instanceN.toml` | XDG 标准目录（推荐） |
| 3 | `~/Library/Application Support/wakem/config.toml` | `~/Library/Application Support/wakem/config-instanceN.toml` | macOS 标准目录 |

### Linux (Wayland)

| 优先级 | 路径（实例 0） | 路径（实例 N） | 说明 |
|:------:|----------------|----------------|------|
| 1 | `~/.wakem.toml` | `~/.wakem-instanceN.toml` | 用户主目录（向后兼容） |
| 2 | `~/.config/wakem/config.toml` | `~/.config/wakem/config-instanceN.toml` | XDG 标准目录（推荐） |

> 注：wakem 当前主要支持 **Windows** 平台（完整功能），**macOS** 平台正在积极开发中（基础架构已完成），Linux (wayland) 支持计划后续迁移。

## 快捷键符号

| 符号 | 按键 |
|:----:|:----:|
| <kbd>Hyper</kbd> | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Meta</kbd> |
| <kbd>HyperShift</kbd> | <kbd>Hyper</kbd>+<kbd>Shift</kbd> |

## 基本配置

```toml
# 基本设置
log_level = "info"        # 日志级别: trace, debug, info, warn, warning, error
tray_icon = true          # 是否显示系统托盘图标
auto_reload = true        # 是否自动重新加载配置
icon_path = "assets/icon.ico"  # 自定义托盘图标路径（可选）

# 键盘重映射（HashMap 格式：源按键 = 目标按键）
[keyboard.remap]
CapsLock = "Backspace"
RightAlt = "Ctrl"

# 导航层 - 按住 CapsLock 时（HashMap 格式）
[keyboard.layers.navigation]
activation_key = "CapsLock"
mode = "Hold"  # Hold: 按住激活, Toggle: 切换激活

[keyboard.layers.navigation.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"

# 窗口管理快捷键（HashMap 格式）
[window.shortcuts]
"Ctrl+Alt+Win+C" = "Center"

# 移动到边缘
"Ctrl+Alt+Win+Home" = "MoveToEdge(Left)"
"Ctrl+Alt+Win+End" = "MoveToEdge(Right)"
"Ctrl+Alt+Win+PageUp" = "MoveToEdge(Top)"
"Ctrl+Alt+Win+PageDown" = "MoveToEdge(Bottom)"

# 半屏显示
"Ctrl+Alt+Win+Shift+Left" = "HalfScreen(Left)"
"Ctrl+Alt+Win+Shift+Right" = "HalfScreen(Right)"

# 循环调整
"Ctrl+Alt+Win+Left" = "LoopWidth(Left)"
"Ctrl+Alt+Win+Right" = "LoopWidth(Right)"

# 窗口切换
"Alt+Grave" = "SwitchToNextWindow"
```

## 全局设置

| 选项 | 类型 | 默认值 | 说明 |
|-----|------|-------|------|
| `log_level` | string | "info" | 日志级别（trace/debug/info/warn/warning/error） |
| `tray_icon` | bool | true | 显示系统托盘图标 |
| `auto_reload` | bool | true | 自动重新加载配置 |
| `icon_path` | string | null | 自定义托盘图标路径（默认尝试加载程序目录下 assets/icon.ico） |

## 键盘配置

### 基础重映射

格式: `源按键 = "目标按键"`

```toml
[keyboard.remap]
CapsLock = "Backspace"
RightAlt = "Ctrl"
```

支持的目标类型：
- **普通按键**: `"Backspace"`, `"Escape"`, `"Enter"` 等
- **修饰键组合**: `"Ctrl+Alt+Meta"` （将 CapsLock 映射为 Hyper 键）

常见用途:
- **CapsLock 改为 Backspace**: 更符合人体工程学
- **CapsLock 改为 Ctrl+Alt+Meta**: 将 CapsLock 变成 Hyper 键
- **RightAlt 改为 Ctrl**: 方便单手操作

### 层系统

层允许你创建上下文相关的键位映射。

```toml
# 定义层（使用点分隔的表名）
[keyboard.layers.层名称]
activation_key = "激活键"
mode = "Hold"  # Hold: 按住激活, Toggle: 切换激活

# 层内的映射
[keyboard.layers.层名称.mappings]
H = "Left"
J = "Down"
```

**模式说明**:
- `Hold`: 按住激活键时层激活，松开后恢复（默认）
- `Toggle`: 按一次激活，再按一次关闭

层内可以映射到：
- **普通按键**: `H = "Left"`
- **组合键**: `W = "Ctrl+Right"` （下一个单词）
- **窗口动作**: `Q = "Center"`

> **验证规则**: `activation_key` 不能为空，否则配置校验会失败。

### 上下文感知快捷键

上下文感知快捷键允许你为特定应用程序定义专属快捷键。

```toml
[[keyboard.context_mappings]]
context = { process_name = "chrome.exe" }
mappings = { CapsLock = "Backspace", "Ctrl+H" = "ShowNotification(Browser, History)" }

[[keyboard.context_mappings]]
context = { process_name = "code.exe" }
mappings = { CapsLock = "Esc", "Ctrl+P" = "ShowNotification(VSCode, Quick Open)" }

# 使用通配符匹配多个编辑器
[[keyboard.context_mappings]]
context = { process_name = "*edit*.exe" }
mappings = { "Ctrl+S" = "ShowNotification(Editor, Save)" }

# 窗口标题匹配（如 YouTube）
[[keyboard.context_mappings]]
context = { window_title = "*YouTube*" }
mappings = { Space = "ShowNotification(YouTube, Play/Pause)" }

# 可执行文件路径匹配
[[keyboard.context_mappings]]
context = { executable_path = "C:\\Program Files\\JetBrains\\*" }
mappings = { "Ctrl+Shift+A" = "ShowNotification(JetBrains, Find Action)" }
```

### 上下文条件字段

| 字段 | 类型 | 说明 |
|-----|------|------|
| `process_name` | string | 进程名匹配，支持通配符 `*` 和 `?` |
| `window_class` | string | 窗口类名匹配 |
| `window_title` | string | 窗口标题匹配 |
| `executable_path` | string | 可执行文件路径匹配 |

**说明**：上下文规则优先级高于全局规则。通配符匹配已完整实现（支持 `*` 匹配任意字符序列和 `?` 匹配单个字符），且匹配大小写不敏感。连续的 `*` 会被合并处理。

## 窗口管理配置

### 窗口切换设置

```toml
[window.switch]
ignore_minimal = true           # 是否忽略最小化的窗口（默认: true）
only_current_desktop = true     # 是否仅在当前虚拟桌面切换（默认: true）
```

### 窗口管理动作

| 动作 | 参数 | 说明 | 示例快捷键 |
|-----|------|------|-----------|
| `Center` | 无 | 窗口居中 | <kbd>Hyper</kbd>+<kbd>C</kbd> |
| `MoveToEdge` | `Left/Right/Top/Bottom` | 移动到屏幕边缘 | <kbd>Hyper</kbd>+<kbd>Home/End/PgUp/PgDn</kbd> |
| `HalfScreen` | `Left/Right/Top/Bottom` | 半屏显示 | <kbd>HyperShift</kbd>+<kbd>方向键</kbd> |
| `LoopWidth` | `Left/Right/Center` | 循环调整宽度 | <kbd>Hyper</kbd>+<kbd>Left/Right</kbd> |
| `LoopHeight` | `Top/Bottom/Center` | 循环调整高度 | <kbd>Hyper</kbd>+<kbd>Up/Down</kbd> |
| `FixedRatio` | `ratio, scale_index` | 固定比例窗口 | <kbd>Hyper</kbd>+<kbd>M</kbd> |
| `NativeRatio` | `scale_index` | 原生比例窗口 | <kbd>HyperShift</kbd>+<kbd>M</kbd> |
| `SwitchToNextWindow` | 无 | 同进程窗口切换 | <kbd>Alt</kbd>+<kbd>`</kbd> |
| `MoveToMonitor` | `Next/Prev/Index` | 跨显示器移动 | <kbd>Hyper</kbd>+<kbd>J/K</kbd> |
| `Minimize` | 无 | 最小化窗口 | - |
| `Maximize` | 无 | 最大化窗口 | - |
| `Restore` | 无 | 还原窗口 | - |
| `Close` | 无 | 关闭窗口 | - |
| `ToggleTopmost` | 无 | 置顶/取消置顶 | - |
| `Move` | `x, y` | 移动窗口到绝对坐标 | - |
| `Resize` | `width, height` | 调整窗口大小 | - |
| `ShowDebugInfo` | 无 | 显示窗口调试信息 | <kbd>Hyper</kbd>+<kbd>W</kbd> |
| `ShowNotification` | `title, message` | 显示通知 | <kbd>HyperShift</kbd>+<kbd>W</kbd> |
| `SavePreset` | `name` | 保存当前窗口为预设 | - |
| `LoadPreset` | `name` | 加载指定预设到当前窗口 | - |
| `ApplyPreset` | 无 | 为当前窗口应用匹配的预设 | - |

### 循环调整尺寸

**宽度循环** (3/4 → 3/5 → 1/2 → 2/5 → 1/4):

```toml
[window.shortcuts]
"Ctrl+Alt+Win+Left" = "LoopWidth(Left)"
"Ctrl+Alt+Win+Right" = "LoopWidth(Right)"
```

**高度循环** (3/4 → 1/2 → 1/4):

```toml
[window.shortcuts]
"Ctrl+Alt+Win+Up" = "LoopHeight(Top)"
"Ctrl+Alt+Win+Down" = "LoopHeight(Bottom)"
```

### 固定比例窗口

保持特定宽高比，循环缩放:

```toml
[window.shortcuts]
# 4:3 比例，从 100% 开始
"Ctrl+Alt+Win+M" = "FixedRatio(1.333, 0)"

# 原生比例（基于屏幕比例），从 90% 开始
"Ctrl+Alt+Win+Shift+M" = "NativeRatio(0)"
```

**参数说明**:
- `FixedRatio`: `ratio` 为宽高比（1.333 = 4:3），`scale_index` 为初始缩放索引
- `NativeRatio`: `scale_index` 为初始缩放索引

连续按键循环: 100% → 90% → 70% → 50% → 100%

### 跨显示器移动

支持按方向或指定显示器索引移动窗口:

```toml
[window.shortcuts]
# 移动到下一台显示器
"Ctrl+Alt+Win+J" = "MoveToMonitor(Next)"
# 移动到上一台显示器
"Ctrl+Alt+Win+K" = "MoveToMonitor(Prev)"
# 移动到指定索引的显示器（从 0 开始）
"Ctrl+Alt+Win+0" = "MoveToMonitor(0)"
"Ctrl+Alt+Win+1" = "MoveToMonitor(1)"
```

## 窗口预设配置

窗口预设允许你保存和恢复特定应用程序的窗口布局。

### 自动应用预设

当窗口创建或激活时，自动应用匹配的预设：

```toml
[window]
auto_apply_preset = true  # 是否自动应用预设（默认: true）

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

### 预设字段说明

| 字段 | 类型 | 必填 | 说明 |
|-----|------|:----:|------|
| `name` | string | 是 | 预设名称 |
| `process_name` | string | 否 | 进程名匹配（支持通配符） |
| `executable_path` | string | 否 | 可执行文件路径匹配（支持通配符） |
| `title_pattern` | string | 否 | 窗口标题匹配模式（支持通配符） |
| `x`, `y` | int | 是 | 窗口位置 |
| `width`, `height` | int | 是 | 窗口大小 |

> 至少需要指定一个匹配条件（`process_name`、`executable_path` 或 `title_pattern`）。

## 鼠标配置

### 按键重映射

```toml
[mouse.button_remap]
# 格式: 源鼠标按键 = 目标鼠标按键
# 支持: Left, Right, Middle, X1, X2
```

### 滚轮设置

```toml
[mouse.wheel]
speed = 3                      # 滚轮速度（默认: 3，必须为正数）
invert = false                 # 反转滚轮方向
acceleration = false           # 启用滚轮加速
acceleration_multiplier = 2.0  # 加速倍数（默认: 2.0，范围: 0.1-10.0）
```

### 水平滚动

按住修饰键时，垂直滚轮变为水平滚动：

```toml
[mouse.wheel.horizontal_scroll]
modifier = "Shift"
step = 1
```

### 音量控制

按住修饰键时，滚轮调节系统音量：

```toml
[mouse.wheel.volume_control]
modifier = "RightAlt"
step = 2
```

### 亮度控制

按住修饰键时，滚轮调节屏幕亮度：

```toml
[mouse.wheel.brightness_control]
modifier = "RightCtrl"
step = 5
```

**支持的修饰键**:
- `Shift`, `LeftShift`, `RightShift`
- `Ctrl`, `Control`, `LeftCtrl`, `RightCtrl`
- `Alt`, `LeftAlt`, `RightAlt`
- `Win`, `Meta`, `Command`

## 网络通信配置

启用网络通信以支持远程控制：

```toml
[network]
enabled = true
bind_address = "127.0.0.1:57427"  # 或使用 instance_id 自动分配
instance_id = 0                    # 实例 ID（范围: 0-255，决定端口号）
auth_key = "your-secret-key-here"  # 认证密钥（如不提供会自动生成）
```

### 安全特性

- 自动拒绝外网连接（只允许 RFC 1918 内网地址）
- 挑战-响应认证（HMAC-SHA256）
- 密钥不在网络上传输
- 启动时如未提供 auth_key 会自动生成随机密钥

### 远程控制示例

```bash
# 在被控制的电脑上启动 wakemd（配置好 auth_key）
wakem daemon

# 在另一台电脑上查看远程状态
wakem --host 192.168.1.100 --auth-key "your-secret-key-here" status

# 重新加载远程配置
wakem --host 192.168.1.100 --auth-key "your-secret-key-here" reload

# 启用/禁用远程映射
wakem --host 192.168.1.100 --auth-key "your-secret-key-here" enable
wakem --host 192.168.1.100 --auth-key "your-secret-key-here" disable
```

## 多实例配置

wakem 支持同时运行多个实例，每个实例有独立的配置和端口。

### 实例配置

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

### 端口分配

- 实例0: 127.0.0.1:57427
- 实例1: 127.0.0.1:57428
- 实例2: 127.0.0.1:57429
- ...（端口 = 57427 + instance_id，端口范围: 1024-65535）

### 使用示例

```bash
# 启动实例0（默认）
wakem daemon

# 启动实例1
wakem daemon --instance 1

# 查看运行中的实例
wakem instances

# 连接到实例1
wakem --instance 1 status
wakem --instance 1 reload

# 启动实例1的托盘客户端
wakem --instance 1
```

## 启动配置

快速启动程序配置支持带参数的命令：

```toml
[launch]
# 格式: "触发键" = "命令"
# 触发键可以是单个键名或完整的快捷键组合

# 简单命令
"Ctrl+Alt+T" = "wt.exe"

# 带参数的命令（使用 Launcher::parse_command 解析）
"Ctrl+Alt+N" = "notepad.exe C:\\Users\\note.txt"
"Ctrl+Alt+G" = "git.exe status"
"Ctrl+Alt+E" = "explorer.exe D:\\"
```

触发键也可以是单独的功能键：

```toml
[launch]
F1 = "notepad.exe"
F2 = "calc.exe"
```

## 宏配置

宏允许你录制一系列键盘和鼠标操作，然后通过快捷键触发。

### 命令行操作

```bash
# 录制宏
wakem record my-macro
# 执行要录制的操作...
# 按 Ctrl+Shift+Esc 停止录制

# 播放宏
wakem play my-macro

# 绑定宏到快捷键
wakem bind-macro my-macro F1

# 列出所有宏
wakem macros

# 删除宏
wakem delete-macro my-macro
```

### 配置文件定义宏

你也可以直接在配置文件中定义宏（使用 MacroStep 格式）：

```toml
# 宏定义（使用 MacroStep 格式）
[macros]
"open-terminal" = [
    { delay_ms = 0, action = { Key = { Press = { scan_code = 91, virtual_key = 91 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 0 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 91, virtual_key = 91 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 10 },
    { delay_ms = 100, action = { Delay = { milliseconds = 100 } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 110 },
]

# 宏触发键绑定
[macro_bindings]
"F1" = "open-terminal"
```

> **验证规则**: `macro_bindings` 中引用的宏名必须在 `[macros]` 中已定义，否则配置校验会失败。空的宏步骤列表不会报错，但会产生警告日志。
>
> **详细文档**: 完整的宏系统文档请参考 [macros.md](macros.md)，包括 MacroStep 格式详细说明、支持的宏动作类型、智能录制特性和按键扫描码参考。

## 按键名称

### 字母键
`A` - `Z`

### 数字键
`0` - `9`

### 功能键
`F1` - `F12`

### 小键盘（Numpad）
`Numpad0` - `Numpad9`
`NumpadDecimal`, `NumpadAdd`, `NumpadSubtract`, `NumpadMultiply`, `NumpadDivide`

### 控制键
- `CapsLock`, `Caps`
- `Shift`, `LeftShift`, `RightShift`
- `Ctrl`, `Control`, `LeftCtrl`, `RightCtrl`
- `Alt`, `LeftAlt`, `RightAlt`
- `Win`, `Meta`, `Command`, `LeftWin`, `RightWin`

### 导航键
- `Up`, `Down`, `Left`, `Right`
- `Home`, `End`
- `PageUp`, `PageDown`
- `Insert`, `Ins`
- `Delete`, `Del`, `ForwardDelete`, `ForwardDel`

### 其他键
- `Backspace`, `Back`
- `Enter`, `Return`
- `Tab`
- `Escape`, `Esc`
- `Space`
- `Grave`, `Backtick` (` 键)
- `Comma` (`,`)
- `Period` (`.`)
- `Equals` (`=`)

## 修饰键语法

在快捷键中使用修饰键:

```toml
# 单修饰键
"Ctrl+C"           # Ctrl + C
"Alt+Tab"          # Alt + Tab
"Win+E"            # Win + E

# 多修饰键（Hyper 键）
"Ctrl+Alt+Win+C"   # Hyper + C
"Ctrl+Alt+Win+Shift+W"  # HyperShift + W

# Meta / Command 作为 Win 的跨平台别名
"Ctrl+Alt+Meta+C"  #等同于 Ctrl+Alt+Win+C（Windows 上 Meta = Win）
"Ctrl+Alt+Command" # 同上
```

**修饰键别名对照表**:

| 通用名称 | 别名 | 平台对应 |
|---------|------|---------|
| `Win` | `Meta`, `Command`, `Cmd` | Windows: Win键, macOS: Command键 |
| `Ctrl` | `Control` | Control 键 |
| `Alt` | - | Alt 键（Windows）/ Option 键（macOS） |
| `Shift` | `LeftShift`, `RightShift` | Shift 键 |

> 注：修饰键名称在配置文件中**不区分大小写**，例如 `ctrl`、`CTRL`、`Ctrl` 均有效。

## 完整配置示例

```toml
# wakem.toml - 完整配置示例

# 基本设置
log_level = "info"
tray_icon = true
auto_reload = true
icon_path = "assets/icon.ico"

# 键盘重映射
[keyboard.remap]
CapsLock = "Backspace"
RightAlt = "Ctrl"

# 导航层
[keyboard.layers.navigation]
activation_key = "CapsLock"
mode = "Hold"

[keyboard.layers.navigation.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"
W = "Ctrl+Right"
B = "Ctrl+Left"

# 窗口管理
[window.shortcuts]
# 窗口居中
"Ctrl+Alt+Win+C" = "Center"
"Ctrl+Alt+Win+Delete" = "Center"

# 移动到边缘
"Ctrl+Alt+Win+Home" = "MoveToEdge(Left)"
"Ctrl+Alt+Win+End" = "MoveToEdge(Right)"
"Ctrl+Alt+Win+PageUp" = "MoveToEdge(Top)"
"Ctrl+Alt+Win+PageDown" = "MoveToEdge(Bottom)"

# 半屏显示（支持四个方向）
"Ctrl+Alt+Win+Shift+Left" = "HalfScreen(Left)"
"Ctrl+Alt+Win+Shift+Right" = "HalfScreen(Right)"
"Ctrl+Alt+Win+Shift+Up" = "HalfScreen(Top)"
"Ctrl+Alt+Win+Shift+Down" = "HalfScreen(Bottom)"

# 循环调整
"Ctrl+Alt+Win+Left" = "LoopWidth(Left)"
"Ctrl+Alt+Win+Right" = "LoopWidth(Right)"
"Ctrl+Alt+Win+Up" = "LoopHeight(Top)"
"Ctrl+Alt+Win+Down" = "LoopHeight(Bottom)"

# 固定比例
"Ctrl+Alt+Win+M" = "FixedRatio(1.333, 0)"
"Ctrl+Alt+Win+Shift+M" = "NativeRatio(0)"

# 窗口切换
"Alt+Grave" = "SwitchToNextWindow"

# 跨显示器
"Ctrl+Alt+Win+J" = "MoveToMonitor(Next)"
"Ctrl+Alt+Win+K" = "MoveToMonitor(Prev)"

# 窗口状态控制
"Ctrl+Alt+Win+N" = "Minimize"
"Ctrl+Alt+Win+X" = "Maximize"
"Ctrl+Alt+Win+Q" = "Close"

# 调试功能
"Ctrl+Alt+Win+W" = "ShowDebugInfo"
"Ctrl+Alt+Win+Shift+W" = "ShowNotification(wakem, Hello World!)"

# 快速启动
[launch]
"Ctrl+Alt+Win+T" = "wt.exe"
"Ctrl+Alt+Win+N" = "notepad.exe"
"Ctrl+Alt+Win+E" = "explorer.exe D:\\"
"Ctrl+Alt+Win+Equals" = "calc.exe"
```

## 配置验证规则

wakem 在加载配置时会进行以下校验，不符合规则的配置会导致启动失败：

| 规则 | 说明 |
|------|------|
| 日志级别 | 必须是 trace/debug/info/warn/warning/error 之一 |
| instance_id | 范围 0-255 |
| 端口号 | 由 instance_id 计算（57427 + instance_id），必须在 1024-65535 范围内 |
| wheel.speed | 必须为正数 |
| acceleration_multiplier | 范围 0.1-10.0 |
| layer.activation_key | 不能为空字符串 |
| macro_bindings | 引用的宏名必须在 `[macros]` 中存在 |

## 故障排除

### 配置不生效

1. 检查配置文件路径是否正确（使用 `wakem config` 打开配置文件夹）
2. 确认 TOML 语法无误（可以使用在线 TOML 验证工具）
3. 查看日志确认配置是否正确加载（设置 `log_level = "debug"`）
4. 尝试手动重载配置: `wakem reload`

### 快捷键冲突

1. 检查是否有其他软件占用相同快捷键
2. 尝试更换快捷键组合
3. 使用更复杂的组合（如三键组合 Hyper）

### 层不生效

1. 检查激活键名称是否正确
2. 确认没有其他软件占用该按键
3. 查看日志确认层是否正确加载

### 窗口管理不生效

1. 检查窗口是否被其他软件锁定
2. 确认窗口不是系统保护窗口（如任务管理器）
3. 查看日志确认命令是否正确发送
4. 某些窗口可能需要以管理员权限运行 wakem

### 通配符不匹配

1. 确保使用正确的通配符语法：`*` 匹配任意字符序列，`?` 匹配单个字符
2. 通配符匹配是大小写不敏感的
3. 连续的 `*` 会被合并处理
