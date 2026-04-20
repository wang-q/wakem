use serde::{Deserialize, Serialize};

/// 按键动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyAction {
    /// 按下按键
    Press { scan_code: u16, virtual_key: u16 },
    /// 释放按键
    Release { scan_code: u16, virtual_key: u16 },
    /// 点击按键（按下并释放）
    Click { scan_code: u16, virtual_key: u16 },
    /// 输入文本
    TypeText(String),
    /// 组合键（如 Ctrl+C）
    Combo {
        modifiers: super::ModifierState,
        key: (u16, u16), // (scan_code, virtual_key)
    },
    /// 无操作
    None,
}

impl KeyAction {
    /// 从 KeyEvent 创建对应的 Press 动作
    pub fn press_from_event(event: &super::KeyEvent) -> Self {
        Self::Press {
            scan_code: event.scan_code,
            virtual_key: event.virtual_key,
        }
    }

    /// 从 KeyEvent 创建对应的 Release 动作
    pub fn release_from_event(event: &super::KeyEvent) -> Self {
        Self::Release {
            scan_code: event.scan_code,
            virtual_key: event.virtual_key,
        }
    }

    /// 创建点击动作
    pub fn click(scan_code: u16, virtual_key: u16) -> Self {
        Self::Click {
            scan_code,
            virtual_key,
        }
    }

    /// 创建按下动作
    pub fn press(scan_code: u16, virtual_key: u16) -> Self {
        Self::Press {
            scan_code,
            virtual_key,
        }
    }

    /// 创建释放动作
    pub fn release(scan_code: u16, virtual_key: u16) -> Self {
        Self::Release {
            scan_code,
            virtual_key,
        }
    }

    /// 创建组合键动作
    pub fn combo(
        modifiers: super::ModifierState,
        scan_code: u16,
        virtual_key: u16,
    ) -> Self {
        Self::Combo {
            modifiers,
            key: (scan_code, virtual_key),
        }
    }
}

/// 鼠标动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseAction {
    /// 移动鼠标
    Move { x: i32, y: i32, relative: bool },
    /// 按下按钮
    ButtonDown { button: super::MouseButton },
    /// 释放按钮
    ButtonUp { button: super::MouseButton },
    /// 点击按钮
    ButtonClick { button: super::MouseButton },
    /// 滚轮滚动
    Wheel { delta: i32 },
    /// 水平滚轮
    HWheel { delta: i32 },
    /// 无操作
    None,
}

/// 边缘枚举（用于窗口管理）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

/// 对齐方式枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Alignment {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

/// 显示器方向枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MonitorDirection {
    Next,
    Prev,
    Index(i32),
}

/// 窗口动作（借鉴 mrw 项目）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowAction {
    /// 窗口居中
    Center,
    /// 移动到屏幕边缘
    MoveToEdge(Edge),
    /// 半屏显示
    HalfScreen(Edge),
    /// 循环调整宽度
    LoopWidth(Alignment),
    /// 循环调整高度
    LoopHeight(Alignment),
    /// 固定比例窗口（比例值，缩放索引）
    FixedRatio { ratio: f32, scale_index: usize },
    /// 原生比例窗口（基于屏幕比例，缩放索引）
    NativeRatio { scale_index: usize },
    /// 同进程窗口切换（Alt+` 功能）
    SwitchToNextWindow,
    /// 跨显示器移动
    MoveToMonitor(MonitorDirection),
    /// 移动窗口（绝对坐标）
    Move { x: i32, y: i32 },
    /// 调整窗口大小
    Resize { width: i32, height: i32 },
    /// 最小化窗口
    Minimize,
    /// 最大化窗口
    Maximize,
    /// 还原窗口
    Restore,
    /// 关闭窗口
    Close,
    /// 置顶/取消置顶
    ToggleTopmost,
    /// 设置透明度
    SetOpacity { opacity: u8 },
    /// 显示调试信息（Hyper+W）
    ShowDebugInfo,
    /// 显示通知（Hyper+Shift+W）
    ShowNotification { title: String, message: String },
    /// 无操作
    None,
}

/// 启动程序动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchAction {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env_vars: Vec<(String, String)>,
}

/// 所有可能的动作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Key(KeyAction),
    Mouse(MouseAction),
    Window(WindowAction),
    Launch(LaunchAction),
    /// 执行多个动作
    Sequence(Vec<Action>),
    /// 无操作
    None,
}

impl Action {
    /// 创建按键动作
    pub fn key(action: KeyAction) -> Self {
        Self::Key(action)
    }

    /// 创建鼠标动作
    pub fn mouse(action: MouseAction) -> Self {
        Self::Mouse(action)
    }

    /// 创建窗口动作
    pub fn window(action: WindowAction) -> Self {
        Self::Window(action)
    }

    /// 创建启动程序动作
    pub fn launch(program: impl Into<String>) -> Self {
        Self::Launch(LaunchAction {
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
            env_vars: Vec::new(),
        })
    }

    /// 创建动作序列
    pub fn sequence(actions: Vec<Action>) -> Self {
        Self::Sequence(actions)
    }

    /// 检查是否是空操作
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}
