# wakem - Window Adjust, Keyboard Enhance, and Mouse

A cross-platform window/keyboard/mouse enhancer.

## Quick Start

Current release: 0.1.1

### 1. Installation

```bash
# Clone the repository
git clone https://github.com/wang-q/wakem.git
cd wakem

# Build
cargo build --release

# Install (optional)
cargo install --path .
```

### 2. Create Configuration File

Copy the example configuration to your config directory:

**Windows:**

```powershell
New-Item -Path "$env:APPDATA\wakem" -ItemType Directory -Force
cp examples/window_manager.toml $env:APPDATA\wakem\config.toml
```

**macOS:**

```bash
mkdir -p ~/Library/Application\ Support/wakem
cp examples/window_manager.toml ~/Library/Application\ Support/wakem/config.toml
```

**Linux:**

```bash
mkdir -p ~/.config/wakem
cp examples/window_manager.toml ~/.config/wakem/config.toml
```

### 3. Start Service

```bash
# One-click start (recommended) - starts both background service and system tray
# On Windows: runs as GUI application (no console window)
wakem

# Or start separately (advanced users)
wakem daemon    # Start background service first
wakem tray      # Then start system tray (assumes background service is running)
```

**Notes:**

- Running `wakem` directly will automatically start both the background service and system tray
- **Windows**: wakem runs as a GUI application (system tray), no console window appears by default
- If the background service is already running, only the system tray will be started
- When the system tray is closed, the background service it started will be automatically stopped
- CLI commands (e.g., `wakem status`, `wakem daemon`, `wakem tray`) will automatically open a
  console window for output/logs
- Use `wakem` (no arguments) for pure GUI mode without console (e.g., for desktop shortcuts)

### 4. Client Commands

```bash
# Global options
--instance, -i    Specify instance ID (default: 0, for multi-instance management)

# Basic commands
wakem status          # View service status
wakem reload          # Reload configuration
wakem save            # Save current configuration to file
wakem enable          # Enable mapping
wakem disable         # Disable mapping
wakem config          # Open configuration folder
wakem instances       # List running instances

# Macro commands
wakem record my-macro        # Start recording macro (press Ctrl+Shift+Esc to stop)
wakem stop-record            # Stop recording macro
wakem play my-macro          # Play macro
wakem macros                 # List all macros
wakem bind-macro my-macro F1 # Bind macro to hotkey
wakem delete-macro my-macro  # Delete macro
```

## Features

### 1. Window Management (Window Adjust)

**Configuration**

See [examples/window_manager.toml](examples/window_manager.toml) for key bindings configuration (
defines Hyper key, shortcuts, etc.).

|        Symbol         |                      Key                      |
|:---------------------:|:---------------------------------------------:|
|   <kbd>hyper</kbd>    | <kbd>ctrl</kbd>+<kbd>opt</kbd>+<kbd>cmd</kbd> |
|                       | <kbd>ctrl</kbd>+<kbd>win</kbd>+<kbd>alt</kbd> |
|                       |              <kbd>capslock</kbd>              |
| <kbd>hyperShift</kbd> |       <kbd>hyper</kbd>+<kbd>shift</kbd>       |

**Movement**

* Center window. <kbd>Hyper</kbd>+<kbd>C</kbd>/<kbd>Delete</kbd>/<kbd>ForwardDelete</kbd>

* Move to edges
    * Left edge - <kbd>Hyper</kbd>+<kbd>Home</kbd>
    * Right edge - <kbd>Hyper</kbd>+<kbd>End</kbd>
    * Top edge - <kbd>Hyper</kbd>+<kbd>PageUp</kbd>
    * Bottom edge - <kbd>Hyper</kbd>+<kbd>PageDown</kbd>

* Move across monitors. <kbd>Hyper</kbd>+<kbd>J</kbd>/<kbd>K</kbd>

**Resize**

* Fixed aspect ratio windows
    * Native aspect ratio (cycle zoom: 0.9, 0.7, 0.5). <kbd>HyperShift</kbd>+<kbd>M</kbd>/<kbd>
      Enter</kbd>
    * 4:3 aspect ratio (cycle zoom: 1.0, 0.9, 0.7, 0.5). <kbd>Hyper</kbd>+<kbd>M</kbd>/<kbd>
      Enter</kbd>

* Width adjustment
    * Cycle ratios: 3/4 → 3/5 → 1/2 → 2/5 → 1/4. <kbd>Hyper</kbd>+<kbd>Left</kbd>/<kbd>Right</kbd>
    * Vertical half-screen. <kbd>HyperShift</kbd>+<kbd>Left</kbd>/<kbd>Right</kbd>

* Height adjustment
    * Cycle ratios: 3/4 → 1/2 → 1/4. <kbd>Hyper</kbd>+<kbd>Up</kbd>/<kbd>Down</kbd>
    * Horizontal half-screen. <kbd>HyperShift</kbd>+<kbd>Up</kbd>/<kbd>Down</kbd>

**Other**

* Switch between same-app windows. <kbd>Alt</kbd>+<kbd>`</kbd>
* Window always-on-top/transparency - configure custom hotkeys

### 2. Keyboard Enhancement (Keyboard Enhance)

- **Key remapping** - CapsLock to Backspace/Esc, swap Ctrl/Alt, CapsLock as Hyper key, etc.
- **Shortcut layer system** - Hold (press and hold to activate) / Toggle (toggle activation) modes
- **Arrow key layer** - CapsLock + H/J/K/L as arrow keys (Vim style)
- **Application shortcuts** - Define exclusive shortcuts for specific applications (context-aware)
- **Quick launch** - Hotkeys to launch commonly used programs (supports commands with parameters)

### 3. Mouse Enhancement (Mouse Enhance)

- **Wheel acceleration** - Automatically increase scroll distance based on scroll speed
- **Horizontal scroll** - Vertical wheel becomes horizontal when holding modifier key
- **Volume control** - Wheel adjusts system volume when holding modifier key
- **Brightness control** - Wheel adjusts screen brightness when holding modifier key
- **Wheel inversion** - Optionally invert wheel direction

### 4. Macro Recording & Playback (Macro)

- **Record macros** - Record keyboard/mouse action sequences, intelligently filtering standalone
  modifier keys
- **Play macros** - Trigger recorded macros via hotkeys or command line
- **Macro management** - View, bind, delete macros, with persistent configuration file storage
- **Modifier key state tracking** - Automatically records and reconstructs modifier key states
  during recording

### 5. Multi-Instance Support

- Run multiple wakem instances simultaneously, each with independent configuration and ports
- Specify instance via `--instance N` parameter
- Automatic port allocation: instance0 = 57427, instance1 = 57428, ...

### 6. Debug Features

* Display window info. <kbd>Hyper</kbd>+<kbd>W</kbd>
* Display test notification. <kbd>HyperShift</kbd>+<kbd>W</kbd>

## Build

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests (171 tests)
cargo test

# Run benchmarks
cargo bench

# Code quality checks
cargo fmt
cargo clippy -- -D warnings
```

## Documentation

- [Configuration Guide](docs/config.md) - Complete keyboard, window management, mouse configuration
  instructions
- [Developer Documentation](docs/developer.md) - Architecture explanation, development plans, and
  API reference
- [Macro System Documentation](docs/macros.md) - Detailed macro recording and playback usage
  instructions

## Reference Projects

- [keymapper](https://github.com/houmain/keymapper) - Cross-platform key remapping tool
- [AutoHotkey](https://www.autohotkey.com/) - Windows automation scripting tool
- [window-switcher](https://github.com/sigoden/window-switcher) - Rust window switching tool
- [mrw](https://github.com/wang-q/mrw) - Personal project, concise window management
- Size looping behavior from [spectacle](https://github.com/eczarny/spectacle).
- Hammerspoon implementation reference
  from [this post](http://songchenwen.com/tech/2015/04/02/hammerspoon-mac-window-manager/).
- AutoHotkey implementation reference from [here](https://github.com/justcla/WindowHotKeys).

## License

MIT License
