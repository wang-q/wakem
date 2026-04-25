# wakem Configuration Guide

This document contains the complete configuration documentation for wakem.

## Configuration File Location

### Windows

wakem uses the following directory structure (following XDG Base Directory specification adapted for Windows):

| Type | Path | Description |
|------|------|------|
| Program | `%LOCALAPPDATA%\Programs\wakem\` | Executable installation directory |
| Config | `%APPDATA%\wakem\` | Configuration file directory |
| Data | `%LOCALAPPDATA%\wakem\` | Log files and other data |
| Startup | `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\` | Startup shortcut |

Configuration file location:

| Path (Instance 0) | Path (Instance N) | Description |
|-------------------|-------------------|-------------|
| `%APPDATA%\wakem\config.toml` | `%APPDATA%\wakem\config-instanceN.toml` | Config directory (XDG-style) |

> `%APPDATA%` typically points to `C:\Users\<Username>\AppData\Roaming`, and `%LOCALAPPDATA%` typically points to `C:\Users\<Username>\AppData\Local`.

### macOS

| Path (Instance 0) | Path (Instance N) | Description |
|-------------------|-------------------|-------------|
| `~/Library/Application Support/wakem/config.toml` | `~/Library/Application Support/wakem/config-instanceN.toml` | macOS standard directory |

### Linux (Wayland)

| Path (Instance 0) | Path (Instance N) | Description |
|-------------------|-------------------|-------------|
| `~/.config/wakem/config.toml` | `~/.config/wakem/config-instanceN.toml` | XDG standard directory |

> Note: wakem currently primarily supports **Windows** platform (full feature set), **macOS** platform is under active development (infrastructure complete), Linux (Wayland) support is planned for future migration.

## Key Symbols

| Symbol | Key Combination |
|:------:|:---------------:|
| <kbd>Hyper</kbd> | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Meta</kbd> |
| <kbd>HyperShift</kbd> | <kbd>Hyper</kbd>+<kbd>Shift</kbd> |

## Basic Configuration

```toml
# Basic settings
log_level = "info"        # Log level: trace, debug, info, warn, warning, error
tray_icon = true          # Show system tray icon
auto_reload = true        # Auto-reload configuration on changes
icon_path = "assets/icon.ico"  # Custom tray icon path (optional)

# Keyboard remapping (HashMap format: source key = target key)
[keyboard.remap]
CapsLock = "Backspace"
RightAlt = "Ctrl"

# Navigation layer - activated when holding CapsLock (HashMap format)
[keyboard.layers.navigation]
activation_key = "CapsLock"
mode = "Hold"  # Hold: activate while held, Toggle: toggle activation

[keyboard.layers.navigation.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"

# Window management shortcuts (HashMap format)
[window.shortcuts]
"Ctrl+Alt+Win+C" = "Center"

# Move to edge
"Ctrl+Alt+Win+Home" = "MoveToEdge(Left)"
"Ctrl+Alt+Win+End" = "MoveToEdge(Right)"
"Ctrl+Alt+Win+PageUp" = "MoveToEdge(Top)"
"Ctrl+Alt+Win+PageDown" = "MoveToEdge(Bottom)"

# Half screen
"Ctrl+Alt+Win+Shift+Left" = "HalfScreen(Left)"
"Ctrl+Alt+Win+Shift+Right" = "HalfScreen(Right)"

# Loop resize
"Ctrl+Alt+Win+Left" = "LoopWidth(Left)"
"Ctrl+Alt+Win+Right" = "LoopWidth(Right)"

# Window switching
"Alt+Grave" = "SwitchToNextWindow"
```

## Global Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `log_level` | string | "info" | Log level (trace/debug/info/warn/warning/error) |
| `tray_icon` | bool | true | Show system tray icon |
| `auto_reload` | bool | true | Automatically reload configuration when changed |
| `icon_path` | string | null | Custom tray icon path (defaults to assets/icon.ico in program directory) |

## Keyboard Configuration

### Basic Remapping

Format: `source_key = "target_key"`

```toml
[keyboard.remap]
CapsLock = "Backspace"
RightAlt = "Ctrl"
```

Supported target types:
- **Regular keys**: `"Backspace"`, `"Escape"`, `"Enter"`, etc.
- **Modifier combinations**: `"Ctrl+Alt+Meta"` (map CapsLock as Hyper key)

Common use cases:
- **CapsLock to Backspace**: More ergonomic typing
- **CapsLock to Ctrl+Alt+Meta**: Turn CapsLock into a Hyper key
- **RightAlt to Ctrl**: Convenient single-hand operation

### Layer System

Layers allow you to create context-sensitive key mappings.

```toml
# Define a layer (using dot-separated table names)
[keyboard.layers.layer_name]
activation_key = "activation_key"
mode = "Hold"  # Hold: activate while held, Toggle: toggle on/off

# Mappings within the layer
[keyboard.layers.layer_name.mappings]
H = "Left"
J = "Down"
```

**Mode descriptions**:
- `Hold`: Layer is active while the activation key is pressed; deactivates on release (default)
- `Toggle`: Press once to activate, press again to deactivate

Mappings within layers can target:
- **Regular keys**: `H = "Left"`
- **Combinations**: `W = "Ctrl+Right"` (next word)
- **Window actions**: `Q = "Center"`

> **Validation rule**: `activation_key` must not be empty, otherwise configuration validation will fail.

### Context-Aware Shortcuts

Context-aware shortcuts allow you to define application-specific shortcuts.

```toml
[[keyboard.context_mappings]]
context = { process_name = "chrome.exe" }
mappings = { CapsLock = "Backspace", "Ctrl+H" = "ShowNotification(Browser, History)" }

[[keyboard.context_mappings]]
context = { process_name = "code.exe" }
mappings = { CapsLock = "Esc", "Ctrl+P" = "ShowNotification(VSCode, Quick Open)" }

# Use wildcards to match multiple editors
[[keyboard.context_mappings]]
context = { process_name = "*edit*.exe" }
mappings = { "Ctrl+S" = "ShowNotification(Editor, Save)" }

# Window title matching (e.g., YouTube)
[[keyboard.context_mappings]]
context = { window_title = "*YouTube*" }
mappings = { Space = "ShowNotification(YouTube, Play/Pause)" }

# Executable path matching
[[keyboard.context_mappings]]
context = { executable_path = "C:\\Program Files\\JetBrains\\*" }
mappings = { "Ctrl+Shift+A" = "ShowNotification(JetBrains, Find Action)" }
```

### Context Condition Fields

| Field | Type | Description |
|-------|------|-------------|
| `process_name` | string | Process name matching, supports wildcards `*` and `?` |
| `window_class` | string | Window class name matching |
| `window_title` | string | Window title matching |
| `executable_path` | string | Executable file path matching |

**Note**: Context rules take higher priority than global rules. Wildcard matching is fully implemented (supports `*` for matching any character sequence and `?` for matching a single character), and matching is case-insensitive. Consecutive `*` characters are merged during processing.

## Window Management Configuration

### Window Switching Settings

```toml
[window.switch]
ignore_minimal = true           # Whether to ignore minimized windows (default: true)
only_current_desktop = true     # Whether to switch only within current virtual desktop (default: true)
```

### Window Management Actions

| Action | Parameters | Description | Example Shortcut |
|--------|-----------|-------------|------------------|
| `Center` | None | Center the window | <kbd>Hyper</kbd>+<kbd>C</kbd> |
| `MoveToEdge` | `Left/Right/Top/Bottom` | Move to screen edge | <kbd>Hyper</kbd>+<kbd>Home/End/PgUp/PgDn</kbd> |
| `HalfScreen` | `Left/Right/Top/Bottom` | Half-screen display | <kbd>HyperShift</kbd>+<kbd>Arrow keys</kbd> |
| `LoopWidth` | `Left/Right/Center` | Cycle width | <kbd>Hyper</kbd>+<kbd>Left/Right</kbd> |
| `LoopHeight` | `Top/Bottom/Center` | Cycle height | <kbd>Hyper</kbd>+<kbd>Up/Down</kbd> |
| `FixedRatio` | `ratio, scale_index` | Fixed aspect ratio window | <kbd>Hyper</kbd>+<kbd>M</kbd> |
| `NativeRatio` | `scale_index` | Native aspect ratio window | <kbd>HyperShift</kbd>+<kbd>M</kbd> |
| `SwitchToNextWindow` | None | Switch between windows of same process | <kbd>Alt</kbd>+<kbd>`</kbd> |
| `MoveToMonitor` | `Next/Prev/Index` | Move across monitors | <kbd>Hyper</kbd>+<kbd>J/K</kbd> |
| `Minimize` | None | Minimize window | - |
| `Maximize` | None | Maximize window | - |
| `Restore` | None | Restore window | - |
| `Close` | None | Close window | - |
| `ToggleTopmost` | None | Toggle always-on-top | - |
| `Move` | `x, y` | Move window to absolute coordinates | - |
| `Resize` | `width, height` | Resize window | - |
| `ShowDebugInfo` | None | Show window debug info | <kbd>Hyper</kbd>+<kbd>W</kbd> |
| `ShowNotification` | `title, message` | Show notification | <kbd>HyperShift</kbd>+<kbd>W</kbd> |
| `SavePreset` | `name` | Save current window as preset | - |
| `LoadPreset` | `name` | Load specified preset to current window | - |
| `ApplyPreset` | None | Apply matched preset to current window | - |

### Loop Resize

**Width cycle** (3/4 → 3/5 → 1/2 → 2/5 → 1/4):

```toml
[window.shortcuts]
"Ctrl+Alt+Win+Left" = "LoopWidth(Left)"
"Ctrl+Alt+Win+Right" = "LoopWidth(Right)"
```

**Height cycle** (3/4 → 1/2 → 1/4):

```toml
[window.shortcuts]
"Ctrl+Alt+Win+Up" = "LoopHeight(Top)"
"Ctrl+Alt+Win+Down" = "LoopHeight(Bottom)"
```

### Fixed Ratio Window

Maintain a specific aspect ratio with cyclic scaling:

```toml
[window.shortcuts]
# 4:3 ratio, starting at 100%
"Ctrl+Alt+Win+M" = "FixedRatio(1.333, 0)"

# Native ratio (based on screen ratio), starting at 90%
"Ctrl+Alt+Win+Shift+M" = "NativeRatio(0)"
```

**Parameter descriptions**:
- `FixedRatio`: `ratio` is the aspect ratio (1.333 = 4:3), `scale_index` is the initial scale index
- `NativeRatio`: `scale_index` is the initial scale index

Continuous key press cycles through: 100% → 90% → 70% → 50% → 100%

### Cross-Monitor Movement

Support moving windows by direction or to a specific monitor index:

```toml
[window.shortcuts]
# Move to next monitor
"Ctrl+Alt+Win+J" = "MoveToMonitor(Next)"
# Move to previous monitor
"Ctrl+Alt+Win+K" = "MoveToMonitor(Prev)"
# Move to specific monitor index (0-based)
"Ctrl+Alt+Win+0" = "MoveToMonitor(0)"
"Ctrl+Alt+Win+1" = "MoveToMonitor(1)"
```

## Window Preset Configuration

Window presets allow you to save and restore specific application window layouts.

### Auto-Apply Presets

When a window is created or activated, automatically apply the matching preset:

```toml
[window]
auto_apply_preset = true  # Whether to auto-apply presets (default: true)

# Define window presets
[[window.presets]]
name = "browser"
process_name = "chrome.exe"
x = 100
y = 100
width = 1200
height = 800

[[window.presets]]
name = "editor"
process_name = "code.exe"
executable_path = "C:\\Program Files\\Microsoft VS Code\\Code.exe"
x = 200
y = 50
width = 1400
height = 900

# Preset shortcuts
[window.shortcuts]
"Ctrl+Alt+S" = "SavePreset(main)"
"Ctrl+Alt+L" = "LoadPreset(main)"
"Ctrl+Alt+A" = "ApplyPreset"
```

### Preset Field Descriptions

| Field | Type | Required | Description |
|-------|------|:--------:|-------------|
| `name` | string | Yes | Preset name |
| `process_name` | string | No | Process name matching (supports wildcards) |
| `executable_path` | string | No | Executable path matching (supports wildcards) |
| `title_pattern` | string | No | Window title pattern matching (supports wildcards) |
| `x`, `y` | int | Yes | Window position |
| `width`, `height` | int | Yes | Window size |

> At least one matching condition must be specified (`process_name`, `executable_path`, or `title_pattern`).

## Mouse Configuration

### Button Remapping

```toml
[mouse.button_remap]
# Format: source mouse button = target mouse button
# Supported: Left, Right, Middle, X1, X2
```

### Wheel Settings

```toml
[mouse.wheel]
speed = 3                      # Wheel speed (default: 3, must be positive)
invert = false                 # Invert wheel direction
acceleration = false           # Enable wheel acceleration
acceleration_multiplier = 2.0  # Acceleration multiplier (default: 2.0, range: 0.1-10.0)
```

### Horizontal Scroll

Hold a modifier key to turn vertical scrolling into horizontal scrolling:

```toml
[mouse.wheel.horizontal_scroll]
modifier = "Shift"
step = 1
```

### Brightness Control

Hold a modifier key to adjust screen brightness with the scroll wheel:

```toml
[mouse.wheel.brightness_control]
modifier = "RightCtrl"
step = 5
```

**Supported modifier keys**:
- `Shift`, `LeftShift`, `RightShift`
- `Ctrl`, `Control`, `LeftCtrl`, `RightCtrl`
- `Alt`, `LeftAlt`, `RightAlt`
- `Win`, `Meta`, `Command`

## Network Communication Configuration

Enable network communication for remote control support:

```toml
[network]
enabled = true
bind_address = "127.0.0.1:57427"  # Or auto-assign based on instance_id
instance_id = 0                    # Instance ID (range: 0-255, determines port number)
auth_key = "your-secret-key-here"  # Authentication key (auto-generated if not provided)
```

### Security Features

- Automatic rejection of external connections (only allows RFC 1918 private addresses)
- Challenge-response authentication (HMAC-SHA256)
- Key is never transmitted over the network
- Random key auto-generated at startup if `auth_key` is not provided

### Remote Control Examples

```bash
# Start wakemd on the controlled machine (configure auth_key)
wakem daemon

# Check remote status from another machine
wakem --host 192.168.1.100 --auth-key "your-secret-key-here" status

# Reload remote configuration
wakem --host 192.168.1.100 --auth-key "your-secret-key-here" reload

# Enable/disable remote mappings
wakem --host 192.168.1.100 --auth-key "your-secret-key-here" enable
wakem --host 192.168.1.100 --auth-key "your-secret-key-here" disable
```

## Multi-Instance Configuration

wakem supports running multiple instances simultaneously, each with independent configuration and ports.

### Instance Configuration

```toml
# Instance 0 config (default):
#   Windows: %APPDATA%\wakem\config.toml
#   macOS: ~/Library/Application Support/wakem/config.toml
#   Linux: ~/.config/wakem/config.toml
[network]
enabled = true
instance_id = 0
auth_key = "instance0-secret"
```

```toml
# Instance 1 config:
#   Windows: %APPDATA%\wakem\config-instance1.toml
#   macOS: ~/Library/Application Support/wakem/config-instance1.toml
#   Linux: ~/.config/wakem/config-instance1.toml
[network]
enabled = true
instance_id = 1
auth_key = "instance1-secret"
```

### Port Allocation

- Instance 0: 127.0.0.1:57427
- Instance 1: 127.0.0.1:57428
- Instance 2: 127.0.0.1:57429
- ... (port = 57427 + instance_id, valid range: 1024-65535)

### Usage Examples

```bash
# Start instance 0 (default)
wakem daemon

# Start instance 1
wakem daemon --instance 1

# List running instances
wakem instances

# Connect to instance 1
wakem --instance 1 status
wakem --instance 1 reload

# Start tray client for instance 1
wakem --instance 1
```

## Launcher Configuration

Quick launch configuration supports commands with arguments:

```toml
[launch]
# Format: "trigger_key" = "command"
# Trigger key can be a single key name or a full shortcut combination

# Simple commands
"Ctrl+Alt+T" = "wt.exe"

# Commands with arguments (parsed by Launcher::parse_command)
"Ctrl+Alt+N" = "notepad.exe C:\\Users\\note.txt"
"Ctrl+Alt+G" = "git.exe status"
"Ctrl+Alt+E" = "explorer.exe D:\\"
```

Trigger keys can also be standalone function keys:

```toml
[launch]
F1 = "notepad.exe"
F2 = "calc.exe"
```

## Macro Configuration

Macros allow you to record a sequence of keyboard and mouse operations, then trigger them via shortcuts.

### Command Line Operations

```bash
# Record a macro
wakem record my-macro
# Perform the actions you want to record...
# Press Ctrl+Shift+Esc to stop recording

# Play a macro
wakem play my-macro

# Bind a macro to a shortcut
wakem bind-macro my-macro F1

# List all macros
wakem macros

# Delete a macro
wakem delete-macro my-macro
```

### Define Macros in Config File

You can also define macros directly in the configuration file (using MacroStep format):

```toml
# Macro definitions (using MacroStep format)
[macros]
"open-terminal" = [
    { delay_ms = 0, action = { Key = { Press = { scan_code = 91, virtual_key = 91 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 0 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 91, virtual_key = 91 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 10 },
    { delay_ms = 100, action = { Delay = { milliseconds = 100 } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 110 },
]

# Macro trigger key bindings
[macro_bindings]
"F1" = "open-terminal"
```

> **Validation rule**: Macro names referenced in `macro_bindings` must already be defined in `[macros]`, otherwise configuration validation will fail. Empty macro step lists will not cause errors but will produce warning logs.
>
> **Detailed documentation**: For complete macro system documentation, refer to [macros.md](macros.md), including detailed MacroStep format descriptions, supported macro action types, smart recording features, and key scan code references.

## Key Names

### Letter Keys
`A` - `Z`

### Number Keys
`0` - `9`

### Function Keys
`F1` - `F12`

### Numpad Keys
`Numpad0` - `Numpad9`
`NumpadDecimal`, `NumpadAdd`, `NumpadSubtract`, `NumpadMultiply`, `NumpadDivide`

### Control Keys
- `CapsLock`, `Caps`
- `Shift`, `LeftShift`, `RightShift`
- `Ctrl`, `Control`, `LeftCtrl`, `RightCtrl`
- `Alt`, `LeftAlt`, `RightAlt`
- `Win`, `Meta`, `Command`, `LeftWin`, `RightWin`

### Navigation Keys
- `Up`, `Down`, `Left`, `Right`
- `Home`, `End`
- `PageUp`, `PageDown`
- `Insert`, `Ins`
- `Delete`, `Del`, `ForwardDelete`, `ForwardDel`

### Other Keys
- `Backspace`, `Back`
- `Enter`, `Return`
- `Tab`
- `Escape`, `Esc`
- `Space`
- `Grave`, `Backtick` (` key)
- `Comma` (`,`)
- `Period` (`.`)
- `Equals` (`=`)

## Modifier Key Syntax

Using modifier keys in shortcuts:

```toml
# Single modifier
"Ctrl+C"           # Ctrl + C
"Alt+Tab"          # Alt + Tab
"Win+E"            # Win + E

# Multiple modifiers (Hyper key)
"Ctrl+Alt+Win+C"   # Hyper + C
"Ctrl+Alt+Win+Shift+W"  # HyperShift + W

# Meta / Command as cross-platform alias for Win
"Ctrl+Alt+Meta+C"  # Equivalent to Ctrl+Alt+Win+C on Windows (Meta = Win)
"Ctrl+Alt+Command" # Same as above
```

**Modifier key alias reference table**:

| Generic Name | Aliases | Platform Mapping |
|-------------|---------|------------------|
| `Win` | `Meta`, `Command`, `Cmd` | Windows: Win key, macOS: Command key |
| `Ctrl` | `Control` | Control key |
| `Alt` | - | Alt key (Windows) / Option key (macOS) |
| `Shift` | `LeftShift`, `RightShift` | Shift key |

> Note: Modifier key names are **case-insensitive** in configuration files, e.g., `ctrl`, `CTRL`, `Ctrl` are all valid.

## Complete Configuration Example

```toml
# wakem.toml - Complete configuration example

# Basic settings
log_level = "info"
tray_icon = true
auto_reload = true
icon_path = "assets/icon.ico"

# Keyboard remapping
[keyboard.remap]
CapsLock = "Backspace"
RightAlt = "Ctrl"

# Navigation layer
[keyboard.layers.navigation]
activation_key = "CapsLock"
mode = "Hold"

[keyboard.layers.navigation.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"
W = "Ctrl+Right"
B = "Ctrl+Left"

# Window management
[window.shortcuts]
# Center window
"Ctrl+Alt+Win+C" = "Center"
"Ctrl+Alt+Win+Delete" = "Center"

# Move to edge
"Ctrl+Alt+Win+Home" = "MoveToEdge(Left)"
"Ctrl+Alt+Win+End" = "MoveToEdge(Right)"
"Ctrl+Alt+Win+PageUp" = "MoveToEdge(Top)"
"Ctrl+Alt+Win+PageDown" = "MoveToEdge(Bottom)"

# Half-screen display (all four directions supported)
"Ctrl+Alt+Win+Shift+Left" = "HalfScreen(Left)"
"Ctrl+Alt+Win+Shift+Right" = "HalfScreen(Right)"
"Ctrl+Alt+Win+Shift+Up" = "HalfScreen(Top)"
"Ctrl+Alt+Win+Shift+Down" = "HalfScreen(Bottom)"

# Cycle resize
"Ctrl+Alt+Win+Left" = "LoopWidth(Left)"
"Ctrl+Alt+Win+Right" = "LoopWidth(Right)"
"Ctrl+Alt+Win+Up" = "LoopHeight(Top)"
"Ctrl+Alt+Win+Down" = "LoopHeight(Bottom)"

# Fixed ratio
"Ctrl+Alt+Win+M" = "FixedRatio(1.333, 0)"
"Ctrl+Alt+Win+Shift+M" = "NativeRatio(0)"

# Window switching
"Alt+Grave" = "SwitchToNextWindow"

# Cross-monitor
"Ctrl+Alt+Win+J" = "MoveToMonitor(Next)"
"Ctrl+Alt+Win+K" = "MoveToMonitor(Prev)"

# Window state control
"Ctrl+Alt+Win+N" = "Minimize"
"Ctrl+Alt+Win+X" = "Maximize"
"Ctrl+Alt+Win+Q" = "Close"

# Debug features
"Ctrl+Alt+Win+W" = "ShowDebugInfo"
"Ctrl+Alt+Win+Shift+W" = "ShowNotification(wakem, Hello World!)"

# Quick launch
[launch]
"Ctrl+Alt+Win+T" = "wt.exe"
"Ctrl+Alt+Win+N" = "notepad.exe"
"Ctrl+Alt+Win+E" = "explorer.exe D:\\"
"Ctrl+Alt+Win+Equals" = "calc.exe"
```

## Configuration Validation Rules

wakem performs the following validations when loading configuration. Invalid configurations will cause startup failure:

| Rule | Description |
|------|-------------|
| Log level | Must be one of trace/debug/info/warn/warning/error |
| instance_id | Range 0-255 |
| Port number | Calculated from instance_id (57427 + instance_id), must be in range 1024-65535 |
| wheel.speed | Must be a positive number |
| acceleration_multiplier | Range 0.1-10.0 |
| layer.activation_key | Must not be an empty string |
| macro_bindings | Referenced macro names must exist in `[macros]` |

## Troubleshooting

### Configuration Not Taking Effect

1. Check that the configuration file path is correct (use `wakem config` to open the config folder)
2. Confirm TOML syntax is correct (you can use online TOML validators)
3. Check logs to confirm configuration loaded correctly (set `log_level = "debug"`)
4. Try manually reloading configuration: `wakem reload`

### Shortcut Conflicts

1. Check if other software is using the same shortcuts
2. Try changing shortcut combinations
3. Use more complex combinations (e.g., three-key combinations like Hyper)

### Layers Not Working

1. Check that the activation key name is correct
2. Confirm no other software is using that key
3. Check logs to confirm the layer loaded correctly

### Window Management Not Working

1. Check if the window is locked by other software
2. Confirm the window is not a system-protected window (e.g., Task Manager)
3. Check logs to confirm commands were sent correctly
4. Some windows may require running wakem with administrator privileges

### Wildcard Matching Issues

1. Ensure correct wildcard syntax: `*` matches any character sequence, `?` matches a single character
2. Wildcard matching is case-insensitive
3. Consecutive `*` characters are merged during processing
