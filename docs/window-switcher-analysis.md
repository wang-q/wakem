# Window Switcher 项目分析

## 项目概述

**Window Switcher** 是一个用 Rust 编写的 Windows 窗口切换增强工具。它提供了类似 macOS 的 Alt+Tab 应用切换和 Alt+` 同应用窗口切换功能。

### 核心特点

1. **应用切换** - Alt+Tab 切换不同应用程序
2. **窗口切换** - Alt+` 在同一应用的多个窗口间切换
3. **可视化界面** - 使用 GDI+ 绘制美观的切换界面
4. **系统托盘** - 支持配置和开机启动
5. **虚拟桌面支持** - 可选仅显示当前虚拟桌面的窗口
6. **UWP 应用支持** - 正确显示 Windows Store 应用图标

---

## 项目架构

### 目录结构

```
window-switcher-1.17.0/
├── src/
│   ├── main.rs           # 程序入口
│   ├── lib.rs            # 库入口
│   ├── app.rs            # 主应用逻辑和消息循环
│   ├── config.rs         # 配置解析
│   ├── keyboard.rs       # 键盘钩子监听
│   ├── painter.rs        # GDI+ 绘制界面
│   ├── foreground.rs     # 前台窗口监控
│   ├── startup.rs        # 开机启动管理
│   ├── trayicon.rs       # 系统托盘图标
│   ├── macros.rs         # 宏定义
│   └── utils/            # 工具模块
│       ├── mod.rs
│       ├── admin.rs      # 管理员权限检测
│       ├── app_icon.rs   # 应用图标获取
│       ├── check_error.rs # Win32 错误检查
│       ├── handle_wrapper.rs # 句柄包装器
│       ├── regedit.rs    # 注册表操作
│       ├── scheduled_task.rs # 计划任务
│       ├── single_instance.rs # 单实例限制
│       ├── window.rs     # 窗口操作
│       ├── windows_theme.rs # 主题检测
│       └── windows_version.rs # Windows 版本
├── assets/
│   └── icon.ico          # 程序图标
├── window-switcher.ini   # 默认配置文件
└── Cargo.toml
```

### 技术栈

- **语言**: Rust
- **Windows API**: windows-rs crate
- **配置**: INI 格式 (rust-ini)
- **日志**: log + simple-logging
- **并发**: parking_lot (Mutex)

---

## 核心组件

### 1. 程序入口 (main.rs)

**职责：**
- 初始化日志系统
- 加载配置文件
- 单实例检查
- 启动应用

```rust
fn run() -> Result<()> {
    let config = load_config().unwrap_or_default();
    // 设置日志...
    let instance = SingleInstance::create("WindowSwitcherMutex")?;
    if !instance.is_single() {
        bail!("Another instance is running...")
    }
    start(&config)
}
```

### 2. 主应用 (app.rs)

**职责：**
- 创建 Windows 窗口
- 消息循环处理
- 窗口/应用切换逻辑
- 状态管理

**核心数据结构：**

```rust
pub struct App {
    hwnd: HWND,                    // 主窗口句柄
    is_admin: bool,                // 是否管理员运行
    trayicon: Option<TrayIcon>,    // 托盘图标
    startup: Startup,              // 启动管理
    config: Config,                // 配置
    switch_windows_state: SwitchWindowsState,  // 窗口切换状态
    switch_apps_state: Option<SwitchAppsState>, // 应用切换状态
    cached_icons: HashMap<String, HICON>, // 图标缓存
    painter: GdiAAPainter,         // 绘制器
}

pub struct SwitchAppsState {
    pub apps: Vec<(HICON, HWND)>,  // (图标, 窗口句柄) 列表
    pub index: usize,              // 当前选中索引
}
```

**消息处理：**

```rust
fn handle_message(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Result<LRESULT> {
    match msg {
        WM_USER_SWITCH_APPS => { /* 切换应用 */ }
        WM_USER_SWITCH_APPS_DONE => { /* 确认切换 */ }
        WM_USER_SWITCH_WINDOWS => { /* 切换窗口 */ }
        WM_USER_SWITCH_WINDOWS_DONE => { /* 确认窗口切换 */ }
        WM_LBUTTONUP => { /* 鼠标点击选择 */ }
        // ...
    }
}
```

### 3. 键盘监听 (keyboard.rs)

**职责：**
- 安装低级键盘钩子
- 监听热键组合
- 发送自定义消息触发切换

**实现细节：**

```rust
pub struct KeyboardListener {
    hook: HHOOK,
}

unsafe extern "system" fn keyboard_proc(
    code: i32, w_param: WPARAM, l_param: LPARAM
) -> LRESULT {
    let kbd_data: &KBDLLHOOKSTRUCT = &*(l_param.0 as *const _);
    
    // 检查修饰键状态
    if state.hotkey.modifier.contains(&scan_code) {
        state.is_modifier_pressed = is_key_pressed();
    }
    
    // 触发切换
    if is_key_pressed() && state.is_modifier_pressed && scan_code == state.hotkey.code {
        SendMessageW(WINDOW, WM_USER_SWITCH_APPS, None, Some(LPARAM(reverse)));
        return LRESULT(1); // 抑制原按键
    }
    
    CallNextHookEx(None, code, w_param, l_param)
}
```

**热键格式：**
- 修饰键: Alt (左右均可)
- 应用切换: Alt + Tab
- 窗口切换: Alt + `
- 支持 Shift 反向切换

### 4. 配置系统 (config.rs)

**配置结构：**

```rust
pub struct Config {
    pub trayicon: bool,                    // 显示托盘图标
    pub log_level: LevelFilter,            // 日志级别
    pub log_file: Option<PathBuf>,         // 日志文件路径
    pub switch_windows_hotkey: Hotkey,     // 窗口切换热键
    pub switch_windows_blacklist: HashSet<String>, // 黑名单
    pub switch_windows_ignore_minimal: bool,
    pub switch_apps_enable: bool,          // 启用应用切换
    pub switch_apps_hotkey: Hotkey,        // 应用切换热键
    pub switch_apps_override_icons: IndexMap<String, String>, // 图标覆盖
}

pub struct Hotkey {
    pub id: u32,
    pub name: String,
    pub modifier: [u32; 2],  // 修饰键扫描码 [左, 右]
    pub code: u32,           // 主键扫描码
}
```

**配置文件示例 (window-switcher.ini)：**

```ini
trayicon = yes

[log]
level = info
path = window-switcher.log

[switch-windows]
hotkey = alt + `
blacklist = ApplicationFrameHost.exe
ignore_minimal = no
only_current_desktop = yes

[switch-apps]
enable = yes
hotkey = alt + tab
ignore_minimal = no
override_icons = firefox.exe=custom.ico
only_current_desktop = yes
```

### 5. 绘制系统 (painter.rs)

**职责：**
- 使用 GDI+ 绘制切换界面
- 支持圆角矩形（Windows 11 风格）
- 自适应主题（深色/浅色）
- 图标渲染

**核心实现：**

```rust
pub struct GdiAAPainter {
    token: usize,          // GDI+ 初始化令牌
    hwnd: HWND,            // 窗口句柄
    hdc_screen: HDC,       // 屏幕 DC
    rounded_corner: bool,  // 是否圆角
    show: bool,
}

impl GdiAAPainter {
    pub fn paint(&mut self, state: &SwitchAppsState) {
        // 计算布局
        let coord = Coordinate::new(state.apps.len() as i32);
        
        // 创建内存 DC
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let bitmap_mem = CreateCompatibleBitmap(hdc_screen, width, height);
        
        // GDI+ 绘制
        let mut graphics = GpGraphics::default();
        GdipCreateFromHDC(hdc_mem, &mut graphics_ptr as _);
        GdipSetSmoothingMode(graphics_ptr, SmoothingModeAntiAlias);
        
        // 绘制背景
        if self.rounded_corner {
            draw_round_rect(graphics_ptr, bg_brush_ptr, ...);
        }
        
        // 绘制图标
        draw_icons(state, hdc_screen, ...);
        
        // 更新分层窗口
        UpdateLayeredWindow(hwnd, None, None, ...);
    }
}
```

**视觉效果：**
- 背景色: 深色 `#4c4c4c` / 浅色 `#e0e0e0`
- 前景色: 深色 `#3b3b3b` / 浅色 `#f2f2f2`
- 图标大小: 64px
- 边框: 10px
- 圆角半径: Windows 11 下为 item_size / 4

### 6. 窗口工具 (utils/window.rs)

**职责：**
- 枚举和过滤窗口
- 获取窗口信息（标题、进程、图标）
- 窗口状态检测
- 前台窗口设置

**关键函数：**

```rust
// 获取窗口状态
pub fn get_window_state(hwnd: HWND) -> (bool, bool, bool, bool) {
    // (is_visible, is_iconic, is_tool, is_topmost)
}

// 检测窗口是否被隐藏（虚拟桌面）
pub fn is_cloaked_window(hwnd: HWND, only_current_desktop: bool) -> bool {
    let cloak_type = get_window_cloak_type(hwnd);
    // DWM_CLOAKED_SHELL 表示在其他虚拟桌面
}

// 设置前台窗口（处理最小化窗口）
pub fn set_foreground_window(hwnd: HWND) {
    if is_iconic_window(hwnd) {
        ShowWindow(hwnd, SW_RESTORE);
    }
    // 发送虚拟鼠标输入以获取焦点
    SendInput(&[input], ...);
    SetForegroundWindow(hwnd);
}

// 列出所有窗口
pub fn list_windows(...) -> IndexMap<String, Vec<(HWND, String)>> {
    // 按进程分组返回窗口列表
}
```

### 7. 图标获取 (utils/app_icon.rs)

**职责：**
- 从可执行文件提取图标
- 支持 UWP 应用（解析 AppxManifest.xml）
- 图标覆盖配置
- 缓存机制

**实现亮点：**

```rust
pub fn get_app_icon(
    override_icons: &IndexMap<String, String>,
    module_path: &str,
    hwnd: HWND,
) -> HICON {
    // 1. 检查配置覆盖
    if let Some((_, path)) = override_icons.iter().find(...) {
        if let Some(icon) = load_image_as_hicon(path) {
            return icon;
        }
    }
    
    // 2. UWP 应用特殊处理
    if module_path.starts_with("C:\\Program Files\\WindowsApps") {
        if let Some(path) = get_appx_logo_path(module_path) {
            return load_image_as_hicon(&path);
        }
    }
    
    // 3. 从 EXE 获取图标
    get_exe_icon(module_path)
        .or_else(|| get_window_icon(hwnd))
        .unwrap_or_else(fallback_icon)
}

// 解析 UWP 应用清单
fn get_appx_logo_path(module_path: &str) -> Option<PathBuf> {
    let manifest_path = module_dir.join("AppxManifest.xml");
    // 解析 XML 获取 Logo 路径
    // 尝试不同尺寸: targetsize-256 > targetsize-128 > scale-200 > scale-100
}
```

### 8. 前台监控 (foreground.rs)

**职责：**
- 监听前台窗口变化
- 检测当前窗口是否在黑名单中
- 用于禁用特定应用的窗口切换

```rust
pub static mut IS_FOREGROUND_IN_BLACKLIST: bool = false;

unsafe extern "system" fn win_event_proc(...) {
    let exe = get_window_exe(hwnd)?.to_lowercase();
    IS_FOREGROUND_IN_BLACKLIST = BLACKLIST.get().unwrap().contains(&exe);
}
```

### 9. 启动管理 (startup.rs)

**职责：**
- 普通用户: 使用注册表 Run 键
- 管理员: 使用计划任务
- 避免权限冲突

```rust
impl Startup {
    pub fn toggle(&mut self) -> Result<()> {
        match (self.is_admin, self.is_enable) {
            (true, true) => delete_scheduled_task(TASK_NAME),
            (true, false) => create_scheduled_task(TASK_NAME, &exe_path),
            (false, true) => reg_disable(),
            (false, false) => reg_enable(&exe_path),
        }
    }
}
```

---

## 关键技术点

### 1. 低级键盘钩子

使用 `WH_KEYBOARD_LL` 钩子监听全局键盘事件：

```rust
SetWindowsHookExW(
    WH_KEYBOARD_LL,
    Some(keyboard_proc),
    Some(hinstance.into()),
    0,
)
```

**注意：** 必须保持消息循环活跃，否则钩子会被系统移除。

### 2. 分层窗口 (Layered Window)

使用 `WS_EX_LAYERED` 样式创建透明窗口：

```rust
CreateWindowExW(
    WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
    ...
)

// 使用 UpdateLayeredWindow 更新内容
UpdateLayeredWindow(hwnd, hdc_screen, None, &size, hdc_mem, ...);
```

### 3. 虚拟桌面检测

使用 DWM API 检测窗口是否在当前虚拟桌面：

```rust
DwmGetWindowAttribute(
    hwnd,
    DWMWA_CLOAKED,
    &mut cloak_type as *mut u32 as *mut c_void,
    size_of::<u32>() as u32,
);

// DWM_CLOAKED_SHELL 表示窗口在其他虚拟桌面
```

### 4. 单实例实现

使用命名互斥量：

```rust
let handle = unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) }?;
let is_single = windows::core::Error::from_win32().code() 
    != ERROR_ALREADY_EXISTS.to_hresult();
```

### 5. GDI+ 抗锯齿绘制

```rust
GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
GdipSetInterpolationMode(graphics, InterpolationModeHighQualityBicubic);
```

---

## 与 wakem 的对比

| 特性 | Window Switcher | wakem (计划) |
|------|----------------|-------------|
| 语言 | Rust | C++ |
| 定位 | 窗口切换增强 | 窗口/键盘/鼠标增强 |
| 功能范围 | 专注窗口切换 | 更广泛 |
| 配置 | INI 文件 | 类似 keymapper.conf |
| 热键系统 | 硬编码 + 配置 | 完全可配置 |
| 可视化 | GDI+ 界面 | 计划中 |
| 跨应用切换 | 是 | 计划中 |
| 代码量 | 精简 (~2k 行) | 待定 |

---

## 可借鉴的设计

### 1. 模块化工具函数

utils 目录的良好组织：
- `window.rs` - 窗口操作
- `app_icon.rs` - 图标获取
- `regedit.rs` - 注册表
- `admin.rs` - 权限检测

### 2. 配置驱动

INI 格式简单易用，支持热键自定义、黑名单、图标覆盖等。

### 3. 图标缓存

使用 `HashMap<String, HICON>` 缓存图标，避免重复加载。

### 4. 权限适配

根据是否管理员选择不同的启动方式（注册表 vs 计划任务）。

### 5. 错误处理

使用 `anyhow` 进行错误传播，统一错误处理风格。

### 6. 主题适配

检测系统主题色，自动切换深色/浅色界面。

---

## 代码统计

- **总代码量**: ~2000 行
- **核心模块**: app, keyboard, painter, config
- **依赖数量**: 9 个主要依赖
- **构建产物**: 单文件可执行程序 (~1MB)

---

## 参考链接

- GitHub: https://github.com/sigoden/window-switcher
- crates.io: https://crates.io/crates/window-switcher
