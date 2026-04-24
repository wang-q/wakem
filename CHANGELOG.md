# Change Log

## Unreleased - ReleaseDate

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
