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

### 1. 窗口管理 (Window Adjust)

|        Symbol         |                      Key                      |
| :-------------------: | :-------------------------------------------: |
|   <kbd>hyper</kbd>    | <kbd>ctrl</kbd>+<kbd>opt</kbd>+<kbd>cmd</kbd> |
|                       | <kbd>ctrl</kbd>+<kbd>win</kbd>+<kbd>alt</kbd> |
| <kbd>hyperShift</kbd> |       <kbd>hyper</kbd>+<kbd>shift</kbd>       |

**移动**

* 窗口居中. <kbd>Hyper</kbd>+<kbd>C</kbd>/<kbd>Delete</kbd>/<kbd>ForwardDelete</kbd>

* 移动到边缘
    * 左边缘 - <kbd>Hyper</kbd>+<kbd>Home</kbd>
    * 右边缘 - <kbd>Hyper</kbd>+<kbd>End</kbd>
    * 上边缘 - <kbd>Hyper</kbd>+<kbd>PageUp</kbd>
    * 下边缘 - <kbd>Hyper</kbd>+<kbd>PageDown</kbd>

* 跨显示器移动. <kbd>Hyper</kbd>+<kbd>J</kbd>/<kbd>K</kbd>

**调整大小**

* 固定比例窗口
    * 原生比例窗口（循环缩放: 0.9, 0.7, 0.5）. <kbd>HyperShift</kbd>+<kbd>M</kbd>/<kbd>Enter</kbd>
    * 4:3 比例窗口（循环缩放: 1.0, 0.9, 0.7, 0.5）. <kbd>Hyper</kbd>+<kbd>M</kbd>/<kbd>Enter</kbd>

* 宽度调整
    * 循环比例: 3/4 → 3/5 → 1/2 → 2/5 → 1/4. <kbd>Hyper</kbd>+<kbd>Left</kbd>/<kbd>Right</kbd>
    * 垂直半屏. <kbd>HyperShift</kbd>+<kbd>Left</kbd>/<kbd>Right</kbd>

* 高度调整
    * 循环比例: 3/4 → 1/2 → 1/4. <kbd>Hyper</kbd>+<kbd>Up</kbd>/<kbd>Down</kbd>
    * 水平半屏. <kbd>HyperShift</kbd>+<kbd>Up</kbd>/<kbd>Down</kbd>

**其他**

* 同应用窗口切换. <kbd>Alt</kbd>+<kbd>`</kbd>
* 窗口置顶/透明 - 配置自定义快捷键

### 2. 键盘增强 (Keyboard Enhance)

- **键位重映射** - CapsLock 改 Backspace/Esc、交换 Ctrl/Alt 等
- **快捷键层** - 按住特定键（如 CapsLock/右Alt）切换快捷键层
- **方向键层** - CapsLock + I/J/K/L 作为方向键
- **应用快捷键** - 为特定应用定义专属快捷键
- **快速启动** - 快捷键启动常用应用

### 3. 鼠标增强 (Mouse Enhance)

- **滚轮增强** - 滚轮在标签页/音量/亮度间切换
- **按键重映射** - 鼠标侧键自定义功能

### 4. 调试功能

* 显示窗口信息. <kbd>Hyper</kbd>+<kbd>W</kbd>
* 显示测试通知. <kbd>HyperShift</kbd>+<kbd>W</kbd>

## 文档

- [键盘配置指南](docs/KEYBOARD.md)
- [窗口管理配置](docs/WINDOW.md)
- [完整配置参考](docs/CONFIG.md)
- [开发文档](docs/developer.md) - 架构说明和开发计划

## 配置文件示例

```toml
# wakem.toml - 窗口管理、键盘增强配置

# 键盘重映射
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

# 窗口管理
[window]
shortcuts = [
    { "Ctrl+Alt+Win+C" = "Center" },
    { "Ctrl+Alt+Win+Left" = "LoopWidth(Left)" },
    { "Ctrl+Alt+Win+Right" = "LoopWidth(Right)" },
]
```

更多配置示例请参考 [完整配置参考](docs/CONFIG.md)。

## 参考项目

- [mrw](https://github.com/wang-q/mrw) - 个人项目，简洁窗口管理
- [keymapper](https://github.com/houmain/keymapper) - 跨平台键位映射工具
- [AutoHotkey](https://www.autohotkey.com/) - Windows 自动化脚本工具
- [window-switcher](https://github.com/sigoden/window-switcher) - Rust 窗口切换工具

## License

MIT License
