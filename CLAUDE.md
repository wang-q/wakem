# CLAUDE.md

此文件是我（AI 助手）在本仓库工作时的行为准则。所有规则都是硬性要求，除非用户明确覆盖。

## 语言规则

- **与用户交流**: 中文
- 本文件 (`CLAUDE.md`) : 使用中文编写
- **代码注释 (doc comments `///` `//!` 和行内 `//`)**: 英文
- **Git 提交信息**: 英文
- **文档正文** (如 `docs/*.md` 中的说明文字): 英文

## 代码风格

### 必须遵守

- 每个 PR / commit 跑 `cargo fmt` 和 `cargo clippy -- -D warnings`，clean 之后再提交
- 日志用 `tracing` 宏 (`info!`, `debug!`, `warn!`, `error!`)，**禁止** `println!` / `eprintln!`
- 公共 API (pub fn / pub struct / pub trait) 必须写 doc comment (英文，一行即可)
- 不写冗余注释 — 如果函数名和类型签名已经说明了行为，不要画蛇添足
- 用 `anyhow::Result<T>` 做函数返回值，`anyhow::bail!` / `anyhow::anyhow!` 构造错误

### 禁止

- 不要引入新依赖，除非用户明确要求
- 不要为了"可能"的未来需求写抽象 — 三次相似代码出现之后再考虑提取
- 不要写半成品实现 — stub / TODO 必须有明确的后续任务链接
- 不要用 `unsafe`，除非有充分理由且用户同意
- 不要写超过一行的 doc comment，除非是 trait 定义或复杂不变量
- 不要反向兼容的 shim（rename `_vars`、re-export 旧类型等）

### 模式参考

项目最近的重构模式（commit 历史）:
- 把大文件拆成小模块（如 `runtime/window_actions.rs` 从 `mapper.rs` 拆出）
- trait 的 default method 委托给 `_inner` 方法，平台只实现 `_inner`
- 公共逻辑提取到 `*_common.rs`，平台模块只保留平台特定 API 调用
- 用 `PlatformFactory` trait + associated types 集中管理平台对象创建

## 项目结构速查

```
src/
├── main.rs          # CLI 入口, clap parse, 日志初始化
├── lib.rs           # 库根, pub mod 声明
├── cli.rs           # clap 命令定义
├── client.rs        # IPC 客户端 (向 daemon 发命令)
├── commands/        # 各 CLI 子命令的实现
├── daemon.rs        # 守护进程主循环
├── config.rs        # TOML 配置解析/验证/热重载
├── constants.rs     # 全局常量 (IPC 端口, 超时等)
├── shutdown.rs      # 优雅关闭信号
├── tray.rs          # 系统托盘入口
├── runtime_util.rs  # 运行时工具函数
├── ipc/             # IPC: 消息, 服务端, 客户端, 认证, 发现, 安全, 限流
├── platform/        # 平台抽象层
│   ├── traits.rs    # 所有平台 trait 定义
│   ├── types.rs     # 平台共享类型
│   ├── mock.rs      # 测试用 mock
│   ├── *_common.rs  # 跨平台公共逻辑
│   ├── windows/     # Windows 实现
│   └── macos/       # macOS 实现
├── runtime/         # 运行时: mapper, layer_manager, macro_player, window_actions
└── types/           # 核心类型: action, input, key_codes, layer, mapping, macros, context
```

## 构建与测试

```bash
# 开发构建
cargo build

# 发布构建 (LTO)
cargo build --release

# 运行所有测试
cargo test

# 按前缀筛选测试
cargo test ut_       # 单元测试
cargo test it_       # 集成测试
cargo test prop_     # 属性测试
cargo test platform_ # 平台特定测试
cargo test e2e_      # 端到端测试

# 单个测试文件
cargo test --test ut_core_daemon

# 代码质量
cargo fmt
cargo clippy -- -D warnings
```

测试文件在 `tests/` 目录下，命名规范: `ut_<模块>.rs` / `it_<描述>.rs` / `prop_<描述>.rs` / `platform_<平台>_<描述>.rs` / `e2e_<平台>_<描述>.rs`。

## 架构规则

### 模块访问

- 核心类型通过 `use wakem::types::*` 导入
- 平台 trait 通过 `use wakem::platform::traits::*` 导入
- 不要在代码中用 `crate::platform::windows::` 或 `crate::platform::macos::` 直接引用（main.rs 和 lib.rs 除外）

### 错误处理

- 库代码返回 `Result<T>` (anyhow)
- 不要在库代码中 `unwrap()` / `expect()` — 用 `?` 传播
- main.rs 和测试中可以 `unwrap()`

### 修改代码时的检查清单

1. 读相关文件，理解现有模式
2. 按照已有模式修改，不要"顺便重构"
3. 修改后跑 `cargo clippy -- -D warnings`
4. 跑相关测试 (`cargo test <prefix>`)
5. 如果改了公共 API，检查所有调用点是否需要更新

## 当前状态

`wakem` (Window Adjust, Keyboard Enhance, and Mouse) 是一个跨平台的输入增强和窗口管理工具。

- **主平台**: Windows (完整支持)
- **次平台**: macOS (开发中)
- **Linux**: 计划中 (占位)
- **版本**: 0.1.3
- **仓库**: https://github.com/wang-q/wakem
