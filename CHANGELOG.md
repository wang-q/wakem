# Change Log

## Unreleased - ReleaseDate

### Features

- **CLI**: Added `shutdown` command to gracefully stop the daemon via IPC
- **Security**: Enhanced secure key handling with `zeroize` for authentication keys
- **IPC**: Added protocol version check and improved instance discovery with debug logging

### Bug Fixes

- **Window Switching**: Fixed window cycling to properly iterate through all windows instead of alternating between two when three or more windows exist
- **Key Event Handling**: Fixed key release filtering to allow layer activation key releases
- **Tray**: Improved tray exit handling and icon cleanup on Windows

### Improvements

- **Architecture**: Major refactoring with platform factory pattern and trait-based architecture
- **Window Management**: Restructured into modular traits (WindowOperations, WindowStateQueries, MonitorOperations, ForegroundWindowOperations)
- **Input Handling**: Consolidated common input device operations and improved key code handling
- **Tray**: Unified menu handling across platforms with heartbeat monitoring for daemon connection
- **Config**: Streamlined error handling in logging initialization
- **Code Quality**: Removed dead code, unused imports, and improved documentation

### Refactoring

- Consolidated platform-specific implementations into common modules
- Moved window action execution logic to separate `window_actions.rs` module
- Extracted common daemon runtime utilities to `runtime_util.rs`
- Reorganized IPC module structure and removed unused message types
- Unified mock window API across platforms
- Simplified trait implementations using macros where appropriate

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
