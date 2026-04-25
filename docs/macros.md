# wakem Macro System Documentation

This document provides a detailed introduction to wakem's macro recording and playback system.

## Table of Contents

- [Overview](#overview)
- [Command Line Usage](#command-line-usage)
- [Defining Macros in Configuration File](#defining-macros-in-configuration-file)
- [Core Components](#core-components)
- [Smart Recording Features](#smart-recording-features)
- [Getting Key Scan Codes](#getting-key-scan-codes)
- [Macro Binding System](#macro-binding-system)

## Overview

The macro system allows users to record and playback keyboard/mouse action sequences. You can:

- Record arbitrary keyboard and mouse operations (intelligently filtering standalone modifier keys)
- Trigger recorded macros via hotkeys or command line
- Precisely define macro steps in the configuration file (using MacroStep format)
- Support all action types (keys, mouse, window management, launching programs, delays, etc.)
- Persist macro data to the configuration file

## Command Line Usage

### Recording Macros

```bash
# Start recording a macro
wakem record my-macro
# Perform the actions you want to record...
# Press Ctrl+Shift+Esc to stop recording

# Stop recording (alternative method)
wakem stop-record
```

After recording is complete, the macro will be automatically saved to the configuration file, and a notification will be displayed to inform you of the recording result.

### Playing Macros

```bash
# Play a macro
wakem play my-macro
```

A notification will be displayed after playback is complete.

### Managing Macros

```bash
# List all macros
wakem macros

# Bind a macro to a hotkey
wakem bind-macro my-macro F1

# Delete a macro
wakem delete-macro my-macro
```

## Defining Macros in Configuration File

You can also define macros directly in the configuration file (using MacroStep format):

```toml
# Macro definitions (using MacroStep format)
[macros]
# Open terminal (Win+R, type wt, Enter)
"open-terminal" = [
    { delay_ms = 0, action = { Key = { Press = { scan_code = 91, virtual_key = 91 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 0 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 91, virtual_key = 91 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 10 },
    { delay_ms = 100, action = { Delay = { milliseconds = 100 } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 110 },
    { delay_ms = 0, action = { Key = { Press = { scan_code = 19, virtual_key = 82 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 120 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 19, virtual_key = 82 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 130 },
    { delay_ms = 100, action = { Delay = { milliseconds = 100 } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 230 },
    { delay_ms = 0, action = { Key = { Press = { scan_code = 20, virtual_key = 84 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 240 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 20, virtual_key = 84 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 250 },
    { delay_ms = 0, action = { Key = { Press = { scan_code = 28, virtual_key = 13 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 260 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 28, virtual_key = 13 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 270 },
]

# Copy and paste (with Ctrl modifier)
"copy-paste" = [
    { delay_ms = 0, action = { Key = { Press = { scan_code = 46, virtual_key = 67 } } }, modifiers = { ctrl = true, shift = false, alt = false, meta = false }, timestamp = 0 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 46, virtual_key = 67 } } }, modifiers = { ctrl = true, shift = false, alt = false, meta = false }, timestamp = 10 },
    { delay_ms = 100, action = { Delay = { milliseconds = 100 } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 110 },
    { delay_ms = 0, action = { Key = { Press = { scan_code = 47, virtual_key = 86 } } }, modifiers = { ctrl = true, shift = false, alt = false, meta = false }, timestamp = 120 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 47, virtual_key = 86 } } }, modifiers = { ctrl = true, shift = false, alt = false, meta = false }, timestamp = 130 },
]

# Window management macro example: center and resize the current window
"center-and-resize" = [
    # First center the window
    { delay_ms = 0, action = { Window = "Center" }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 0 },
    { delay_ms = 200, action = { Delay = { milliseconds = 200 } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 200 },
]

# Launch program macro example
"launch-browser" = [
    { delay_ms = 0, action = { Launch = { program = "chrome.exe", args = [], working_dir = null, env_vars = [] } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 0 },
]

# Macro trigger key bindings
[macro_bindings]
"F1" = "open-terminal"
"Ctrl+Shift+V" = "copy-paste"
```

### MacroStep Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `delay_ms` | u64 | Delay time before this step (milliseconds) |
| `action` | Action | The action to perform |
| `modifiers` | ModifierState | Modifier key state during recording (ctrl/shift/alt/meta) |
| `timestamp` | u64 | Event timestamp (for debugging/analysis) |

### Macro Action Types

The macro system reuses the `Action` enum and supports all action types:

#### Key Actions (KeyAction)

| Action | Description | Parameters |
|--------|-------------|------------|
| `Press` | Press a key | `scan_code`, `virtual_key` |
| `Release` | Release a key | `scan_code`, `virtual_key` |
| `Click` | Click a key (press and release) | `scan_code`, `virtual_key` |
| `TypeText` | Type a text string | `String` |
| `Combo` | Combo key (with modifiers) | `modifiers` (ModifierState), `key` (scan_code, virtual_key) |
| `None` | No operation | - |

#### Mouse Actions (MouseAction)

| Action | Description | Parameters |
|--------|-------------|------------|
| `Move` | Move mouse | `x`, `y`, `relative` |
| `ButtonDown` | Press button | `button` (Left/Right/Middle/X1/X2) |
| `ButtonUp` | Release button | `button` |
| `ButtonClick` | Click button | `button` |
| `Wheel` | Vertical wheel scroll | `delta` (positive value scrolls up) |
| `HWheel` | Horizontal wheel scroll | `delta` (positive value scrolls right) |
| `None` | No operation | - |

#### Window Actions (WindowAction)

The following window management actions are supported:

**Basic Operations**

| Action | Description |
|--------|-------------|
| `Center` | Center window |
| `Minimize` | Minimize window |
| `Maximize` | Maximize window |
| `Restore` | Restore window |
| `Close` | Close window |
| `ToggleTopmost` | Toggle always-on-top |

**Position and Size**

| Action | Description | Parameters |
|--------|-------------|------------|
| `MoveToEdge(Edge)` | Move to screen edge | Left/Right/Top/Bottom |
| `HalfScreen(Edge)` | Half-screen display | Left/Right/Top/Bottom |
| `LoopWidth(Alignment)` | Loop resize width | Left/Right/Center/Top/Bottom |
| `LoopHeight(Alignment)` | Loop resize height | Left/Right/Center/Top/Bottom |
| `FixedRatio { ratio, scale_index }` | Fixed ratio window | Ratio value, scale index |
| `NativeRatio { scale_index }` | Native ratio window | Scale index |
| `Move { x, y }` | Move window to absolute coordinates | x, y coordinates |
| `Resize { width, height }` | Resize window | Width, height |

**Advanced Features**

| Action | Description | Parameters |
|--------|-------------|------------|
| `SwitchToNextWindow` | Switch between windows of same process (Alt+`) | - |
| `MoveToMonitor(MonitorDirection)` | Move across monitors | Next/Prev/Index(n) |
| `ShowDebugInfo` | Show debug info | - |
| `ShowNotification { title, message }` | Show notification | Title, content |
| `SavePreset { name }` | Save current window as preset | Preset name |
| `LoadPreset { name }` | Load specified preset to current window | Preset name |
| `ApplyPreset` | Apply matched preset to current window | - |
| `None` | No operation | - |

#### Launch Actions (LaunchAction)

| Field | Type | Description |
|-------|------|-------------|
| `program` | String | Program path or name |
| `args` | Vec\<String\> | Command line argument list |
| `working_dir` | Option\<String\> | Working directory (null means not specified) |
| `env_vars` | Vec\<(String, String)\> | Environment variable key-value pair list |

#### Other Actions

| Action | Description | Parameters |
|--------|-------------|------------|
| `Sequence` | Action sequence (nest multiple actions) | `Vec<Action>` |
| `Delay` | Delay wait | `milliseconds` (u64) |
| `None` | No operation | - |

## Core Components

| Component | File | Description |
|-----------|------|-------------|
| `MacroRecorder` | `src/types/macros.rs` | Record input events, using `Action::from_input_event()` |
| `MacroPlayer` | `src/runtime/macro_player.rs` | Playback macro actions, supports modifier state reconstruction |
| `MacroManager` | `src/types/macros.rs` | Macro manager, responsible for loading, adding, deleting, querying macro definitions |
| `MacroStep` | `src/types/macros.rs` | Macro step structure, containing action, modifiers, timestamp |
| `Macro` | `src/types/macros.rs` | Macro definition structure, containing name, step list, metadata |
| `ModifierState` | `src/types/mod.rs` | Modifier key state structure (ctrl/shift/alt/meta) |
| `Action` | `src/types/action.rs` | Unified action enum |

### Architecture Overview

```
┌─────────────────────────────────────────┐
│           MacroRecorder                 │
│  - Uses Action::from_input_event()      │
│  - Uses is_modifier() to filter         │
│    standalone modifier keys             │
│  - Uses from_virtual_key() + merge()    │
│    to track modifier state              │
│  - Records as Vec<MacroStep>            │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│          simplify_delays()              │
│                                         │
│  Merges consecutive short delays (<50ms)│
│  Retains necessary delays (>=50ms)      │
│  Generates final Vec<MacroStep>         │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│         MacroManager                    │
│  - load_from_config(): Load from config │
│  - add_macro() / remove_macro()         │
│  - get_macro() / get_macro_names()      │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│           MacroPlayer                   │
│                                         │
│  Iterate through MacroStep:             │
│  1. Execute delay_ms delay              │
│  2. ensure_modifiers() reconstructs     │
│     modifier state                      │
│  3. execute_action() calls handler:     │
│     - Key -> send_key_action()          │
│     - Mouse -> send_mouse_action()      │
│     - Window -> (via ActionMapper)      │
│     - Launch -> launcher                │
│     - System -> system_control          │
│     - Sequence -> recursive processing  │
│     - Delay / None -> sleep or skip     │
│  4. release_all_modifiers() cleanup     │
└─────────────────────────────────────────┘
```

### Data Structures

```rust
// Modifier key state
pub struct ModifierState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,  // Win key / Command key
}

impl ModifierState {
    // Create modifier state from virtual key code
    pub fn from_virtual_key(key: u16, pressed: bool) -> Option<(Self, bool)>;
    // Merge another modifier state
    pub fn merge(&mut self, other: &ModifierState);
    // Check if no modifiers are pressed
    pub fn is_empty(&self) -> bool;
}

// Macro step
pub struct MacroStep {
    pub delay_ms: u64,           // Delay (milliseconds)
    pub action: Action,          // Action
    pub modifiers: ModifierState, // Modifier state during recording
    pub timestamp: Timestamp,     // Timestamp
}

// Macro definition
pub struct Macro {
    pub name: String,              // Macro name
    pub steps: Vec<MacroStep>,    // Step list
    pub created_at: Option<String>, // Creation time
    pub description: Option<String>, // Description (optional)
}
```

### Recording Flow Architecture

```
┌─────────────────────────────────────────┐
│           MacroRecorder                 │
│                                         │
│  Input Event ──► is_modifier()?         │
│              │                          │
│         ┌────┴────┐                     │
│        Yes       No                     │
│         │         │                     │
│        Skip  from_input_event()         │
│              │                          │
│              ▼                          │
│      update_modifiers()                 │
│      (from_virtual_key + merge)         │
│              │                          │
│              ▼                          │
│      Create MacroStep                   │
│      Add to recording buffer            │
└─────────────────┬───────────────────────┘
                  │ stop_recording()
                  ▼
┌─────────────────────────────────────────┐
│          simplify_delays()              │
│                                         │
│  MIN_DELAY_MS = 50ms                    │
│  - Adjacent step interval < 50ms:       │
│    no delay inserted                    │
│  - Adjacent step interval >= 50ms:      │
│    insert Delay                         │
│  - Explicit Delay action always kept    │
└─────────────────────────────────────────┘
```

### Playback Flow Architecture

```
┌─────────────────────────────────────────┐
│           MacroPlayer::play_macro()     │
│                                         │
│  for step in macro.steps:               │
│  ├─ 1. sleep(delay_ms)                  │
│  ├─ 2. ensure_modifiers(&step.modifiers)│
│  │   Press Ctrl -> Alt -> Meta -> Shift │
│  ├─ 3. execute_action(&step.action)     │
│  │   ├─ Key    -> send_key_action()     │
│  │   ├─ Mouse  -> send_mouse_action()   │
│  │   ├─ Window -> (log)                 │
│  │   ├─ Launch -> (log)                 │
│  │   ├─ System -> (log)                 │
│  │   ├─ Sequence -> recursive sub-action│
│  │   └─ Delay/None -> skip or sleep     │
│  └─ end for                             │
│                                         │
│  release_all_modifiers()                │
│  Release order: Meta -> Alt -> Shift -> Ctrl│
└─────────────────────────────────────────┘
```

## Smart Recording Features

### 1. Filtering Standalone Modifier Keys

Automatically skip standalone Ctrl/Shift/Alt/Win keys during recording, only recording combo keys. For example:

- Recording `Ctrl+C`: Only records 2 steps (C press and release), not 4
- Pressing `Ctrl` alone: Completely skipped, nothing recorded

Implementation: Determine if it's a modifier key via the `KeyEvent::is_modifier()` method. Judgment is based on virtual key code matching:

| Modifier | Virtual Key Code (including left/right variants) |
|----------|--------------------------------------------------|
| Shift | 0x10, 0xA0 (left), 0xA1 (right) |
| Ctrl | 0x11, 0xA2 (left), 0xA3 (right) |
| Alt | 0x12, 0xA4 (left), 0xA5 (right) |
| Win/Meta | 0x5B (left), 0x5C (right) |

### 2. Tracking Modifier State

Record the complete modifier state when each action occurs:

```rust
pub struct ModifierState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,  // Win key
}
```

This is very important for correctly restoring context during playback. During recording, modifier key events are parsed via `ModifierState::from_virtual_key()` and merged into the current state using the `merge()` method.

### 3. Delay Optimization

Automatically merge short delays (< 50ms), only retaining meaningful delays:

```rust
const MIN_DELAY_MS: u64 = 50; // Minimum delay threshold
```

For example:
- Press A (0ms) -> Release A (10ms) -> Press B (90ms): Only insert an ~80ms delay between A and B
- Press A (0ms) -> Release A (10ms) -> **Delay(100ms)** -> Press B: Retain explicit 100ms delay

## Getting Key Scan Codes

If you need to manually write macro configurations, you may need to know the scan code and virtual key code for specific keys.

For a complete list of key names and scan code/virtual key code对照表, please refer to [keys.md](keys.md).

### Methods to Get Scan Codes

1. **Use wakem logs**: Set `log_level = "debug"` and start the daemon, check key information in the logs
2. **Online tools**: Use Windows Virtual-Key Codes online query tools
3. **MSDN documentation**: Refer to [Virtual-Key Codes (Winuser.h)](https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes)

## Macro Binding System

Macros can be triggered via hotkeys, which need to be configured in the `[macro_bindings]` section:

```toml
[macros]
"my-macro" = [ ... ]  # Macro definition

[macro_bindings]
"F1" = "my-macro"          # F1 key trigger
"Ctrl+Shift+V" = "my-macro"  # Ctrl+Shift+V trigger
```

### Binding Rules

1. The bound trigger key must be a valid key name or shortcut format
2. The macro referenced by the binding must exist in `[macros]` (checked during config validation)
3. One macro can be bound to multiple trigger keys
4. Deleting a macro will also delete related bindings
5. Macros with empty steps will not error, but will produce warning logs

### Configuration Validation

The following validations are performed when loading configuration (see `Config::validate()`):

- Macro names referenced in `[macro_bindings]` must exist in `[macros]`
- Macros with empty steps will output warning log prompts

### Binding Examples

```toml
[macros]
# Quickly open commonly used applications
"open-terminal" = [
    { delay_ms = 0, action = { Launch = { program = "wt.exe", args = [], working_dir = null, env_vars = [] } }, ... },
]
"open-explorer" = [
    { delay_ms = 0, action = { Launch = { program = "explorer.exe", args = [], working_dir = null, env_vars = [] } }, ... },
]
"open-browser" = [
    { delay_ms = 0, action = { Launch = { program = "chrome.exe", args = [], working_dir = null, env_vars = [] } }, ... ],
]

[macro_bindings]
# F series Function key bindings
"F1" = "open-terminal"
"F2" = "open-explorer"
"F3" = "open-browser"

# Combo key bindings
"Ctrl+Alt+T" = "open-terminal"
```

---

For more configuration information, please refer to [config.md](config.md). For development-related information, please refer to [developer.md](developer.md).
