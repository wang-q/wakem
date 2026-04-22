# wakem - Window Adjust, Keyboard Enhance, and Mouse

一个跨平台的窗口管理、键盘增强、鼠标增强工具。借鉴 [mrw](https://github.com/wang-q/mrw)、[keymapper](https://github.com/houmain/keymapper) 和 [AutoHotkey](https://www.autohotkey.com/) 的优秀设计。

## 快速开始

### 1. 安装

```bash
# 克隆仓库
git clone https://github.com/wang-q/wakem.git
cd wakem

# 构建
cargo build --release

# 安装（可选）
cargo install --path .
```

### 2. 创建配置文件

复制示例配置到用户目录：

**Windows:**

```powershell
cp examples/window_manager.toml $env:USERPROFILE\.wakem.toml
```

**macOS:**

```bash
cp examples/window_manager.toml ~/.wakem.toml
```

### 3. 启动服务

```bash
# 启动守护进程（后台服务，用于窗口管理和宏录制等功能）
wakem daemon

# 运行系统托盘客户端（默认命令，仅提供图形界面）
wakem
# 或
wakem tray
```

**注意**：
- 守护进程 (`wakem daemon`) 是后台服务，处理键盘映射、窗口管理等功能
- 系统托盘客户端 (`wakem tray`) 提供图形界面，需要守护进程已在运行
- 建议先启动守护进程，再启动托盘客户端以获得完整功能

### 4. 客户端命令

```bash
# 全局选项
--instance, -i    指定实例ID（默认: 0，用于多实例管理）

# 基础命令
wakem status          # 查看服务状态
wakem reload          # 重载配置
wakem save            # 保存当前配置到文件
wakem enable          # 启用映射
wakem disable         # 禁用映射
wakem config          # 打开配置文件夹
wakem instances       # 列出运行中的实例

# 宏命令
wakem record my-macro        # 开始录制宏（按 Ctrl+Shift+Esc 停止）
wakem stop-record            # 停止录制宏
wakem play my-macro          # 播放宏
wakem macros                 # 列出所有宏
wakem bind-macro my-macro F1 # 绑定宏到快捷键
wakem delete-macro my-macro  # 删除宏
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

- **键位重映射** - CapsLock 改 Backspace/Esc、交换 Ctrl/Alt、CapsLock 改为 Hyper 键等
- **快捷键层系统** - Hold（按住激活）/ Toggle（切换激活）两种模式
- **方向键层** - CapsLock + H/J/K/L 作为方向键（Vim 风格）
- **应用快捷键** - 为特定应用程序定义专属快捷键（上下文感知）
- **快速启动** - 快捷键启动常用程序（支持带参数的命令）

### 3. 鼠标增强 (Mouse Enhance)

- **滚轮加速** - 根据滚动速度自动增加滚动距离
- **水平滚动** - 按住修饰键时垂直滚轮变为水平滚动
- **音量控制** - 按住修饰键时滚轮调节系统音量
- **亮度控制** - 按住修饰键时滚轮调节屏幕亮度
- **滚轮反转** - 可选反转滚轮方向

### 4. 宏录制回放 (Macro)

- **录制宏** - 录制键盘/鼠标操作序列，智能过滤单独修饰键
- **播放宏** - 通过快捷键或命令行触发录制的宏
- **宏管理** - 查看、绑定、删除宏，配置文件持久化存储
- **修饰键状态跟踪** - 录制时自动记录和重建修饰键状态

### 5. 多实例支持

- 同时运行多个 wakem 实例，每个实例独立配置和端口
- 通过 `--instance N` 参数指定实例
- 自动端口分配：实例0 = 57427，实例1 = 57428，...

### 6. 调试功能

* 显示窗口信息. <kbd>Hyper</kbd>+<kbd>W</kbd>
* 显示测试通知. <kbd>HyperShift</kbd>+<kbd>W</kbd>

## 构建

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# 运行测试 (171 tests)
cargo test

# 运行性能基准测试
cargo bench

# 代码质量检查
cargo fmt
cargo clippy -- -D warnings
```

## 文档

- [配置指南](docs/config.md) - 完整的键盘、窗口管理、鼠标等配置说明
- [开发文档](docs/developer.md) - 架构说明、开发计划和 API 参考
- [宏系统文档](docs/macros.md) - 宏录制回放的详细使用说明

## 配置文件示例

```toml
# wakem.toml - 窗口管理、键盘增强配置

# 基本设置
log_level = "info"
tray_icon = true
auto_reload = true
icon_path = "assets/icon.ico"  # 可选

# 键盘重映射（HashMap 格式）
[keyboard.remap]
CapsLock = "Backspace"
RightAlt = "Ctrl"

# 导航层（HashMap 格式）
[keyboard.layers.navigation]
activation_key = "CapsLock"
mode = "Hold"

[keyboard.layers.navigation.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"

# 窗口管理（HashMap 格式）
[window.shortcuts]
"Ctrl+Alt+Win+C" = "Center"
"Ctrl+Alt+Win+Left" = "LoopWidth(Left)"
"Ctrl+Alt+Win+Right" = "LoopWidth(Right)"
"Alt+Grave" = "SwitchToNextWindow"

# 快速启动程序
[launch]
"Ctrl+Alt+Win+T" = "wt.exe"
"Ctrl+Alt+Win+N" = "notepad.exe"
```

更多配置示例请参考 [完整配置参考](docs/config.md)。

## 参考项目

- [mrw](https://github.com/wang-q/mrw) - 个人项目，简洁窗口管理
- [keymapper](https://github.com/houmain/keymapper) - 跨平台键位映射工具
- [AutoHotkey](https://www.autohotkey.com/) - Windows 自动化脚本工具
- [window-switcher](https://github.com/sigoden/window-switcher) - Rust 窗口切换工具

## License

MIT License
