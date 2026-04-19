# AutoHotkey 项目分析

## 项目概述

**AutoHotkey** 是 Windows 平台上最著名的自动化脚本语言和热键工具。它允许用户通过简单的脚本实现键盘/鼠标重映射、窗口管理、自动化任务等功能。

### 核心特点

1. **脚本语言** - 完整的解释型脚本语言，支持变量、函数、对象、类等
2. **热键/热字符串** - 强大的键盘快捷键和文本替换功能
3. **窗口管理** - 窗口查找、操作、控制等功能
4. **自动化** - 模拟键盘鼠标、发送消息、COM 接口等
5. **GUI 支持** - 创建图形用户界面
6. **编译功能** - 可将脚本编译为独立可执行文件

---

## 项目架构

### 目录结构

```
AutoHotkey-2.0.23/
├── source/              # 源代码目录
│   ├── AutoHotkey.cpp   # 程序入口
│   ├── application.cpp  # 消息循环和线程管理
│   ├── script.cpp       # 脚本解析和执行
│   ├── hook.cpp         # 键盘/鼠标钩子
│   ├── hotkey.cpp       # 热键管理
│   ├── window.cpp       # 窗口操作
│   ├── keyboard_mouse.cpp # 键盘鼠标模拟
│   ├── script_gui.cpp   # GUI 实现
│   └── ...
├── resources/           # 资源文件
└── .vscode/            # VS Code 配置
```

### 核心组件

#### 1. 程序入口 (AutoHotkey.cpp)

**职责：**
- 命令行参数解析
- 初始化全局数据
- 加载脚本文件
- 启动消息循环

**关键流程：**
```cpp
_tWinMain()
├── EarlyAppInit()          # 早期初始化
├── ParseCmdLineArgs()      # 解析命令行
├── g_script.LoadFromFile() # 加载脚本
├── CheckPriorInstance()    # 检查单实例
├── InitForExecution()      # 执行前初始化
└── MainExecuteScript()     # 主执行循环
```

#### 2. 应用层 (application.cpp/h)

**职责：**
- Windows 消息循环 (`MsgSleep`)
- 线程管理
- 定时器处理
- 中断控制

**核心机制：**

```cpp
// 消息循环（替代 Sleep，防止钩子事件延迟）
bool MsgSleep(int aSleepDuration, MessageMode aMode);

// 线程中断控制
#define SLEEP_WITHOUT_INTERRUPTION(aSleepTime) \
{\
    g_AllowInterruption = FALSE;\
    MsgSleep(aSleepTime);\
    g_AllowInterruption = TRUE;\
}
```

**设计要点：**
- 使用 `MsgSleep` 替代 `Sleep`，确保钩子事件及时处理
- 线程中断机制防止脚本执行被热键打断
- 定时器轮询实现 `SetTimer` 功能

#### 3. 脚本引擎 (script.cpp/h)

**职责：**
- 脚本解析（词法分析、语法分析）
- 命令执行
- 变量管理
- 函数调用
- 对象系统

**核心数据结构：**

```cpp
class Script {
    // 脚本行数组
    Line *mFirstLine, *mLastLine;
    
    // 变量表
    Var *mVar;
    
    // 热键数组
    Hotkey **mHotkey;
    
    // 窗口组
    WinGroup **mWinGroup;
    
    // 函数表
    UserFunc **mFunc;
    
    // 对象原型
    Object *mObjectPrototype;
};
```

**脚本执行模式：**
```cpp
enum ExecUntilMode {
    NORMAL_MODE,      // 正常执行
    UNTIL_RETURN,     // 执行到 return
    UNTIL_BLOCK_END,  // 执行到块结束
    ONLY_ONE_LINE     // 只执行一行
};
```

#### 4. 钩子系统 (hook.cpp/h)

**职责：**
- 低级键盘钩子 (`WH_KEYBOARD_LL`)
- 低级鼠标钩子 (`WH_MOUSE_LL`)
- 热键触发判断
- 按键事件转发/抑制

**核心数据结构：**

```cpp
struct key_type {
    ToggleValueType *pForceToggle;  // 切换键（如 CapsLock）
    HotkeyIDType hotkey_to_fire_upon_release;  // 释放时触发的热键
    HotkeyIDType first_hotkey;  // 第一个使用此键的热键
    modLR_type as_modifiersLR;  // 作为修饰键的位掩码
    bool used_as_prefix;        // 是否用作前缀键
    bool used_as_suffix;        // 是否用作后缀键
    bool is_down;               // 当前是否按下
    bool down_performed_action; // 按下时是否触发了动作
};
```

**钩子消息类型：**
```cpp
enum UserMessages {
    AHK_HOOK_HOTKEY = WM_USER,    // 热键触发
    AHK_HOTSTRING,                 // 热字符串
    AHK_USER_MENU,                 // 用户菜单
    AHK_DIALOG,                    // 对话框
    AHK_NOTIFYICON,                // 托盘图标
    AHK_CLIPBOARD_CHANGE,          // 剪贴板变化
    AHK_CHANGE_HOOK_STATE,         // 改变钩子状态
    // ...
};
```

#### 5. 热键管理 (hotkey.cpp/h)

**职责：**
- 热键注册/注销
- 热键变体管理（不同条件下触发不同动作）
- 修饰键处理
- Alt-Tab 特殊处理

**热键类型：**
```cpp
enum HotkeyTypeEnum {
    HK_NORMAL,        // 普通热键（使用 RegisterHotkey）
    HK_KEYBD_HOOK,    // 键盘钩子热键
    HK_MOUSE_HOOK,    // 鼠标钩子热键
    HK_BOTH_HOOKS,    // 同时使用两种钩子
    HK_JOYSTICK       // 游戏杆热键
};
```

**热键变体：**
```cpp
struct HotkeyVariant {
    IObjectRef mCallback;           // 回调函数
    HotkeyCriterion *mHotCriterion; // 触发条件（如 #IfWinActive）
    int mPriority;                  // 优先级
    UCHAR mMaxThreads;              // 最大线程数
    bool mEnabled;                  // 是否启用
    // ...
};
```

#### 6. 窗口管理 (window.cpp/h)

**职责：**
- 窗口查找（标题、类名、进程名等）
- 窗口操作（移动、调整大小、关闭等）
- 窗口组管理
- 控件操作

**窗口搜索类：**
```cpp
class WindowSearch {
    DWORD mCriteria;              // 搜索条件类型
    LPCTSTR mCriterionTitle;      // 窗口标题
    LPCTSTR mCriterionClass;      // 窗口类名
    LPCTSTR mCriterionPath;       // 进程路径
    HWND mCriterionHwnd;          // 窗口句柄
    DWORD mCriterionPID;          // 进程 ID
    WinGroup *mCriterionGroup;    // 窗口组
    
    HWND IsMatch(bool aInvert);   // 判断窗口是否匹配
};
```

**匹配模式：**
```cpp
#define FIND_ANYWHERE        1  // 包含匹配
#define FIND_REGEX           2  // 正则匹配
#define FIND_IN_LEADING_PART 3  // 开头匹配
// 默认：精确匹配
```

#### 7. 键盘鼠标模拟 (keyboard_mouse.cpp/h)

**职责：**
- 模拟键盘输入（SendInput）
- 模拟鼠标移动/点击
- 游戏杆支持
- 按键历史记录

**虚拟键定义：**
```cpp
// 自定义虚拟键（用于鼠标滚轮等）
#define VK_NEW_MOUSE_FIRST 0x9A
#define VK_WHEEL_LEFT      0x9C
#define VK_WHEEL_RIGHT     0x9D
#define VK_WHEEL_DOWN      0x9E
#define VK_WHEEL_UP        0x9F

// 扫描码定义
#define SC_LCONTROL 0x01D
#define SC_RCONTROL 0x11D
#define SC_LSHIFT   0x02A
#define SC_RSHIFT   0x136
// ...
```

---

## 关键技术点

### 1. 钩子机制

AutoHotkey 使用 Windows 低级钩子实现全局热键：

```cpp
// 安装钩子
HHOOK g_hKeyboardHook = SetWindowsHookEx(WH_KEYBOARD_LL, 
    LowLevelKeyboardProc, g_hInstance, 0);
HHOOK g_hMouseHook = SetWindowsHookEx(WH_MOUSE_LL, 
    LowLevelMouseProc, g_hInstance, 0);

// 键盘钩子回调
LRESULT CALLBACK LowLevelKeyboardProc(int nCode, WPARAM wParam, 
    LPARAM lParam) {
    KBDLLHOOKSTRUCT *pKbd = (KBDLLHOOKSTRUCT*)lParam;
    // 判断是否为热键，决定是否抑制事件
    if (IsHotkey(pKbd->vkCode, pKbd->scanCode)) {
        PostMessage(g_hWnd, AHK_HOOK_HOTKEY, hotkey_id, 0);
        return 1; // 抑制原事件
    }
    return CallNextHookEx(g_hKeyboardHook, nCode, wParam, lParam);
}
```

### 2. 消息驱动架构

整个程序基于 Windows 消息循环，避免使用 `Sleep`：

```cpp
bool MsgSleep(int aSleepDuration, MessageMode aMode) {
    MSG msg;
    while (GetMessage(&msg, NULL, 0, 0)) {
        // 处理各种 AHK 自定义消息
        switch (msg.message) {
            case AHK_HOOK_HOTKEY:
                FireHotkey(msg.wParam);
                break;
            case AHK_HOTSTRING:
                FireHotstring(msg.wParam);
                break;
            // ...
        }
        
        // 检查定时器
        CHECK_SCRIPT_TIMERS_IF_NEEDED;
        
        // 检查是否达到睡眠时间
        if (SleepDurationExpired())
            break;
    }
}
```

### 3. 线程管理

AutoHotkey 使用协作式多任务，每个热key启动一个"准线程"：

```cpp
struct ScriptThread {
    Line *mCurrentLine;        // 当前执行行
    Line *mJumpToLine;         // 跳转目标
    int mPriority;             // 优先级
    bool mIsCritical;          // 是否关键（不可中断）
    // ...
};

// 线程数组
g_array[MAX_THREADS];
g = g_array + g_nThreads;  // 当前线程指针
```

### 4. 脚本解析

脚本解析分为多个阶段：

1. **词法分析** - 将脚本分割为 token
2. **语法分析** - 识别命令、函数、热键等
3. **代码生成** - 构建 Line 链表
4. **执行** - 遍历 Line 链表执行

```cpp
// 脚本行结构
struct Line {
    ActionType mActionType;    // 动作类型
    ArgStruct mArg[7];         // 参数
    Line *mNextLine;           // 下一行
    Line *mPrevLine;           // 上一行
    // ...
};
```

### 5. 对象系统

AutoHotkey v2 引入了完整的对象系统：

```cpp
class Object : public ObjectBase {
    // 属性
    ObjectMember mMembers[];
    
    // 方法
    ResultType Invoke(IObject *aThis, int aFlags, 
        LPTSTR aName, int aParamCount, 
        ExprTokenType *aParam[], ExprTokenType &aResult);
};

// 原型链
Object::CreateRootPrototypes();  // 创建 Object、Array、Map 等原型
```

---

## 与 wakem 的对比

| 特性 | AutoHotkey | wakem (计划) |
|------|-----------|-------------|
| 定位 | 完整脚本语言 | 配置驱动的工具 |
| 学习曲线 | 较高（需学习语法） | 较低（简单配置） |
| 功能范围 | 非常广泛 | 专注窗口/键盘/鼠标 |
| 性能 | 解释执行 | 原生实现 |
| 配置方式 | 脚本文件 | 类似 keymapper.conf |
| 跨平台 | Windows only | Windows (未来可能扩展) |
| 窗口管理 | 完善 | 重点功能 |
| 热键系统 | 强大 | 参考 keymapper |

---

## 可借鉴的设计

### 1. 消息循环设计

AutoHotkey 的 `MsgSleep` 设计值得借鉴，避免使用 `Sleep` 导致钩子事件延迟。

### 2. 热键优先级和变体

热键变体（不同条件下触发不同动作）和优先级系统非常灵活。

### 3. 窗口搜索机制

多条件窗口搜索（标题、类名、PID、进程名等）和匹配模式设计完善。

### 4. 线程中断控制

`Critical` 和 `AllowInterruption` 机制确保关键操作不被打断。

### 5. 钩子状态管理

动态安装/卸载钩子，只在需要时启用，减少系统开销。

---

## 代码统计

- **总文件数**: ~70 个源文件
- **核心模块**: script, hook, hotkey, window, keyboard_mouse, application
- **代码风格**: C++ 混合 Windows API
- **许可证**: GPL v2

---

## 参考链接

- 官网: https://www.autohotkey.com/
- 文档: https://www.autohotkey.com/docs/
- GitHub: https://github.com/AutoHotkey/AutoHotkey
