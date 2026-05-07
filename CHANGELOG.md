# Change Log

## Unreleased - ReleaseDate

## 0.1.8 - 2026-05-07

### Bug Fixes

- **Hyper Key**: Generalized key suppression to all mapped key combinations, not just Hyper-only combos. Implemented three-layer suppression (hyper keys, hyper suffix keys, physical modifier combos) in the keyboard hook.
- **E2E Tests**: Fixed hanging test caused by Ctrl+C SendInput triggering console SIGINT; fixed residual `` ` `` character in terminal from injected keystrokes accumulating in console input buffer.

## 0.1.7 - 2026-05-07

### Bug Fixes

- **Hyper Key**: Fixed modifier combo (Ctrl+Alt+Meta) not suppressing Backspace/Delete, causing weird characters (`^¿`) to appear
- **Hyper Key**: Fixed CapsLock-as-Hyper suppressing all non-modifier keys (including C, M, J, K), which broke shortcuts like Ctrl+C

### Improvements

- **Windows Input**: Improved hyper key detection to support both registered hyper keys (CapsLock) and modifier combinations (Ctrl+Alt+Meta)
- **Windows Input**: Refined key suppression to only target Backspace (0x08) and Delete (0x2E) when Hyper is active, allowing normal key combinations to pass through
- **Config**: Added modifier counting and rule sorting to ensure correct matching order for shortcuts with different modifier counts

### Testing

- **E2E**: Moved keyboard event tests that change machine state to `e2e_windows_keyboard.rs` with `#[ignore]` attribute
- **E2E**: Added comprehensive E2E tests for modifier combo hyper key behavior
- **Unit**: Added tests for Backspace shortcut parsing and modifier counting

## 0.1.6 - 2026-05-05

### Bug Fixes

- Fixed FixedRatio/NativeRatio window stuck at maximum size on repeated hotkey presses
- Fixed window resizing for applications with minimum size constraints (e.g., Calculator)

### Features

- Added `Backspace` as a center window hotkey for Windows keyboards

## 0.1.5 - 2026-05-05

### Bug Fixes

- **Hyper Key**: Fixed CapsLock as Hyper key producing `@` symbol. Hyper keys now act as pure virtual modifiers without sending actual modifier key events to the system.

### Improvements

- **Daemon**: Hyper key events are now consumed internally and do not trigger mapped actions, preventing unintended output.

## 0.1.4 - 2026-05-04

### Features

- **CLI**: Added `shutdown` command to gracefully stop the daemon via IPC
- **IPC**: Added protocol version check in IPC client
- **Types**: Added `is_subset_of` method to `ModifierState` for hyper key support

### Bug Fixes

- **Windows**: Fixed window switching to properly cycle through all windows instead of alternating between two
- **Daemon**: Fixed key release filtering to allow layer activation key releases
- **IPC**: Fixed IPC bridge loops to use `recv_timeout` instead of `try_recv`

### Improvements

- **Security**: Enhanced secure key handling with `zeroize` for authentication keys
- **IPC**: Added debug logging for instance discovery results
- **Config**: Combined config updates into single write operation
- **Config**: Streamlined error handling in logging initialization
- **Tray**: Improved tray exit handling and icon cleanup on Windows
- **Tray**: Unified menu handling across platforms
- **Daemon**: Updated window action execution to pass services explicitly

### Refactoring

- **Architecture**: Major refactoring with platform factory pattern and trait-based architecture
- **Window Management**: Restructured into modular traits (WindowOperations, WindowStateQueries, MonitorOperations, ForegroundWindowOperations, WindowSwitching)
- **Window Actions**: Moved window action execution logic to separate `window_actions.rs` module
- **Input**: Consolidated common input device operations into common module
- **Platform**: Moved key name parsing to common module
- **Platform**: Standardized key code handling with Windows virtual key codes as internal representation
- **Tray**: Simplified tray API by removing hwnd parameter from register()
- **IPC**: Removed unused message types and timeout
- **Types**: Consolidated window context types
- **Runtime**: Extracted common daemon runtime utilities to `runtime_util.rs`
- **Launcher**: Moved command parsing to `LaunchAction`
- **Notification**: Removed unused notification initialization code
- **Tests**: Unified mock window API across platforms
- **Tests**: Removed obsolete notification-related tests
- **Code Quality**: Replaced `anyhow::anyhow` with `anyhow::bail` for consistency
- **Code Quality**: Removed dead code and unused imports

## 0.1.3 - 2026-04-26

### Improvements

- Improved memory management and thread safety
- Improved error handling and shutdown logic
- Made config file operations non-fatal

### Refactoring

- Consolidated platform-specific implementations
- Reorganized IPC module structure
- Cleaned up unused code and imports

## 0.1.2 - 2026-04-24

### Bug Fixes

- **Launcher**: Fixed hotkey parsing in `[launch]` configuration to support modifier keys (e.g., `Ctrl+Alt+Meta+T`)
- **Windows Tray**: Added timeout and force kill mechanism for daemon shutdown to prevent tray from hanging on exit

### Improvements

- **Testing**: Added E2E tests for Windows launcher functionality
- **Documentation**: Updated developer documentation with launcher test cases

## 0.1.1 - 2026-04-24

### Initial Release

First public release of wakem - a cross-platform input enhancement and window management tool in Rust.

### Features

**Keyboard Enhancement**
- Key remapping with support for single keys and key combinations
- Modifier key customization (Ctrl, Alt, Shift, Meta/Win)
- Hyper key support with virtual modifiers and split press/release actions
- Layer system with hold and toggle modes for contextual key mappings
- Macro recording and playback with delay support

**Mouse Enhancement**
- Mouse button remapping
- Scroll wheel acceleration/deceleration
- Horizontal scroll support

**Window Management**
- Window movement and resizing with customizable steps
- Window maximize, minimize, and fullscreen actions
- Window cycling for multi-window applications
- Preset layouts with auto-scaling and cycle support
- Auto-apply rules based on window class or title patterns
- Topmost window toggling

**Application Launcher**
- Launch applications with keyboard shortcuts
- Support for command-line arguments

**System Integration**
- Daemon mode for background operation
- System tray integration (Windows and macOS)
- Desktop notifications
- Graceful shutdown handling

**Configuration**
- TOML format configuration files
- Hot-reload support for configuration changes
- Multi-instance support with unique identifiers
- Comprehensive configuration validation

### Platform Support

**Windows**
- Raw Input for low-level keyboard/mouse event capture
- SendInput for output simulation
- Win32 API for window management
- Windows-specific tray implementation with custom icons

**macOS**
- CGEvent for input event handling
- Cocoa framework integration
- Accessibility API for window management
- Native system tray with menu support
- Core Graphics for display and coordinate handling

### Architecture

**Core Design**
- Modular architecture with clear separation of concerns
- Platform abstraction layer with trait-based interfaces
- IPC communication using TCP with HMAC-SHA256 authentication
- Rate limiting and IP whitelist for security
- Async runtime with Tokio

**Input/Output System**
- Unified input event abstraction
- Output device traits for cross-platform compatibility
- Mock implementations for testing

**Runtime System**
- Event mapping engine with rule-based lookup
- Layer manager for contextual key mappings
- Macro player with step-based execution

### Testing

**Test Coverage**
- Unit tests for core types and data structures
- Integration tests for cross-module interactions
- Property-based testing using proptest
- Platform-specific tests for Windows and macOS
- Mock implementations for platform-dependent code

**Benchmarks**
- Criterion-based performance benchmarks
- Cross-platform and macOS-specific benchmark suites

### Technical Highlights

- Safe Rust implementation with no unsafe code in core logic
- Structured logging with tracing
- Error handling with custom error types
- Constants module for magic number elimination
- Comprehensive documentation and configuration examples
