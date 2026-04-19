# wakem - Window Adjust, Keyboard Enhance, and Mouse

一个跨平台的窗口管理、键盘增强、鼠标增强工具。借鉴 [mrw](https://github.com/yourusername/mrw)、[keymapper](https://github.com/houmain/keymapper) 和 [AutoHotkey](https://www.autohotkey.com/) 的优秀设计。

## 快速开始

### 1. 安装

```bash
# 克隆仓库
git clone https://github.com/yourusername/wakem.git
cd wakem

# 构建
cargo build --release

# 安装（可选）
cargo install --path .
```

### 2. 创建配置文件

复制示例配置到用户目录：

```bash
cp examples/minimal.toml %USERPROFILE%\wakem.toml
```

最小配置示例 (`wakem.toml`):

```toml
[keyboard]
remap = [
    { from = "CapsLock", to = "Backspace" },
]
```

### 3. 启动服务

```bash
# 启动守护进程（需要管理员权限）
wakem daemon
# 或
wakemd

# 启动客户端（系统托盘）
wakem
# 或
wakem tray
```

### 4. 客户端命令

```bash
wakem status      # 查看服务状态
wakem reload      # 重载配置
wakem enable      # 启用映射
wakem disable     # 禁用映射
wakem config      # 打开配置文件夹
```

## 功能特性

### 1. 窗口调整 (Window Adjust)

| 功能 | 描述 | 状态 |
|------|------|------|
| 窗口移动 | 快捷键移动窗口到屏幕各位置（左半、右半、上半、下半、四角、中心等） | ✅ |
| 窗口调整大小 | 快捷键调整窗口大小（最大化、最小化、1/2屏、1/3屏、1/4屏等） | ✅ |
| 窗口切换 | Alt+` 同应用窗口切换 | ✅ |
| 多显示器支持 | 窗口跨显示器移动、每个显示器独立布局 | ✅ |
| 窗口置顶/透明 | 快捷键设置窗口置顶或调整透明度 | ⏳ |
| 虚拟桌面 | 快捷键切换/移动窗口到不同虚拟桌面 | ⏳ |

**已实现的窗口管理功能**（借鉴 mrw 项目）:
- 窗口居中显示
- 移动到屏幕边缘
- 半屏显示
- 宽度循环调整: 3/4 → 3/5 → 1/2 → 2/5 → 1/4
- 高度循环调整: 3/4 → 1/2 → 1/4
- 固定比例窗口: 任意比例，循环缩放
- 原生比例窗口: 基于屏幕比例
- 跨显示器移动

### 2. 键盘增强 (Keyboard Enhance)

| 功能 | 描述 | 状态 |
|------|------|------|
| 键位重映射 | CapsLock 改 Backspace/Esc、交换 Ctrl/Alt 等 | ✅ |
| 快捷键层 | 按住特定键（如 CapsLock/右Alt）切换快捷键层 | ✅ |
| 方向键层 | CapsLock + I/J/K/L 作为方向键 | ✅ |
| 文本扩展 | 输入缩写自动展开 | ⏳ |
| 应用快捷键 | 为特定应用定义专属快捷键 | ✅ |
| 快速启动 | 快捷键启动常用应用 | ✅ |

### 3. 鼠标增强 (Mouse Enhance)

| 功能 | 描述 | 状态 |
|------|------|------|
| 滚轮增强 | 滚轮在标签页/音量/亮度间切换 | ⏳ |
| 按键重映射 | 鼠标侧键自定义功能 | ⏳ |

## 目录结构

```
wakem/
├── Cargo.toml              # 项目配置
├── src/
│   ├── main.rs             # 统一入口（wakem/wakemd/wakemctl）
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
│   ├── input_test.rs
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

## 配置文件示例

```toml
# wakem.toml - 窗口管理、键盘增强配置

# ============================================
# 键盘重映射
# ============================================
[keyboard]
remap = [
    { from = "CapsLock", to = "Backspace" },
]

# 导航层 - 按住 CapsLock 时
[[keyboard.layers]]
name = "navigation"
activation_key = "CapsLock"
mode = "Hold"
mappings = [
    { from = "H", to = "Left" },
    { from = "J", to = "Down" },
    { from = "K", to = "Up" },
    { from = "L", to = "Right" },
]

# ============================================
# 窗口管理
# ============================================
[window]
enabled = true

[[window.mappings]]
key = "Alt+Enter"
action = "Maximize"

[[window.mappings]]
key = "Alt+`"
action = "SwitchToNextWindow"

[[window.mappings]]
key = "Ctrl+Alt+C"
action = "Center"

[[window.mappings]]
key = "Ctrl+Alt+Left"
action = "HalfScreen(Left)"

[[window.mappings]]
key = "Ctrl+Alt+Right"
action = "HalfScreen(Right)"

# ============================================
# 快速启动
# ============================================
[[keyboard.mappings]]
key = "Win+C"
action = "Launch('wt.exe')"
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

### Phase 4: Windows 鼠标增强 ⏳ 待实现

- [ ] 鼠标事件处理
- [ ] 滚轮增强

> **注**: 不实现鼠标手势功能，使用场景有限且实现复杂

### Phase 5: Windows 完善 ⏳ 进行中

- [x] 系统托盘
- [x] 输入捕获（Raw Input + LLKH）
- [x] 配置重载
- [ ] 启动项管理
- [ ] 错误处理和日志
- [ ] 安装和打包

### Phase 6: macOS 移植 ⏳ 待实现

### Phase 7: Linux 移植 ⏳ 待实现

## 参考项目

| 项目 | 语言 | 核心特点 | 学习重点 |
|------|------|----------|----------|
| [keymapper](https://github.com/houmain/keymapper) | C++ | 跨平台、客户端-服务端架构 | 架构设计、配置语法、输入处理 |
| [AutoHotkey](https://github.com/AutoHotkey/AutoHotkey) | C++ | 完整脚本语言、强大热键系统 | 热键变体、窗口操作、消息循环 |
| [window-switcher](https://github.com/sigoden/window-switcher) | Rust | 精致窗口切换、GDI+ 界面 | 窗口切换 UI、图标获取、虚拟桌面 |
| **mrw** (个人项目) | Lua/AHK | 简洁窗口管理、循环尺寸调整 | 窗口布局算法、多显示器支持 |

## License

MIT License
