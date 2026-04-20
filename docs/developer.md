# wakem 开发文档

本文档包含 wakem 的开发记录、架构说明和开发计划。

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
