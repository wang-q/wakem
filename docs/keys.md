# wakem Key Names Reference

This document lists all key names recognized by wakem's configuration parser (`parse_key()` in `src/config.rs`).
These names can be used in `[keyboard.remap]`, layer mappings, and window shortcuts.

## Table of Contents

- [Usage](#usage)
- [Letter Keys](#letter-keys)
- [Number Keys](#number-keys)
- [Function Keys](#function-keys)
- [Modifier Keys](#modifier-keys)
- [Navigation & Editing Keys](#navigation--editing-keys)
- [Special Keys](#special-keys)
- [Punctuation & Symbol Keys (US Layout)](#punctuation--symbol-keys-us-layout)
- [Numpad Keys](#numpad-keys)
- [Key Name Resolution Order](#key-name-resolution-order)

## Usage

Key names are used throughout the configuration file:

```toml
[keyboard.remap]
CapsLock = "Backspace"

[keyboard.layers.navigation.mappings]
H = "Left"
J = "Down"

[window.shortcuts]
"Ctrl+Alt+C" = "Center"
```

Most keys support multiple name aliases (e.g., both `"Enter"` and `"Return"` work).

---

## Letter Keys

| Name | Scan Code | Virtual Key | Notes |
|------|-----------|-------------|-------|
| a ~ z | 0x1E ~ 0x2C | 0x41 ~ 0x5A (65~90) | Case-insensitive |

**Examples:**
```toml
[keyboard.remap]
a = "b"           # Swap A and B
CapsLock = "ctrl"   # CapsLock acts as Ctrl
```

---

## Number Keys

### Main Keyboard Row

| Name | Scan Code | Virtual Key |
|------|-----------|-------------|
| 0 ~ 9 | 0x0B ~ 0x14 | 0x30 ~ 0x39 (48~57) |

### Numpad (Numeric Keypad)

See [Numpad Keys](#numpad-keys) section below for the full numpad reference.

---

## Function Keys

| Name | Scan Code | Virtual Key |
|------|-----------|-------------|
| f1 ~ f12 | 0x3B ~ 0x58 | 0x70 ~ 0x7B (112~123) |

**Examples:**
```toml
[launch]
F1 = "notepad.exe"
F2 = "calc.exe"
f3 = "explorer.exe"      # Lowercase also works
```

---

## Modifier Keys

Modifier keys are used in two contexts:

1. **As remapping targets**: e.g., `RightAlt = "Ctrl"`
2. **In shortcut triggers**: e.g., `"Ctrl+Alt+C" = "Center"`

### Modifier Key Names (for remapping)

| Name | Alias(es) | Scan Code | Virtual Key | Description |
|------|-----------|-----------|-------------|-------------|
| lshift | — | 0x2A | 0xA0 (160) | Left Shift |
| rshift | — | 0x36 | 0xA1 (161) | Right Shift |
| lctrl / lcontrol | — | 0x1D | 0xA2 (162) | Left Control |
| rctrl / rcontrol | — | 0xE01D | 0xA3 (163) | Right Control |
| lalt | — | 0x38 | 0xA4 (164) | Left Alt |
| ralt | — | 0xE038 | 0xA5 (165) | Right Alt |
| lwin / lmeta | — | 0xE05B | 0x5B (91) | Left Windows / Command |
| rwin / rmeta | — | 0xE05C | 0x5C (92) | Right Windows / Command |

### Shortcut Modifier Prefixes

When used as prefix in shortcut strings (e.g., `"Ctrl+Alt+C"`):

| Prefix | Also accepted as | Description |
|--------|------------------|-------------|
| ctrl | control | Control key |
| alt | — | Alt key |
| shift | — | Shift key |
| win | meta, command, cmd | Windows key / Command key |

**Examples:**
```toml
[keyboard.remap]
RightAlt = "Ctrl"       # Remap Right Alt to Ctrl
CapsLock = "Ctrl+Alt+Meta"  # Hyper key mapping

[window.shortcuts]
"Ctrl+Alt+C" = "Center"
"ctrl+alt+meta+m" = "FixedRatio(1.333, 0)"    # Case-insensitive
"cmd+shift+left" = "HalfScreen(Left)"            # macOS-style alias
```

---

## Navigation & Editing Keys

| Name | Alias(es) | Scan Code | Virtual Key | Description |
|------|-----------|-----------|-------------|-------------|
| enter | return | 0x1C | 0x0D (13) | Enter / Return |
| space | — | 0x39 | 0x20 (32) | Space bar |
| tab | — | 0x0F | 0x09 (9) | Tab |
| backspace | back | 0x0E | 0x08 (8) | Backspace |
| escape | esc | 0x01 | 0x1B (27) | Escape |

### Arrow Keys

| Name | Scan Code | Virtual Key |
|------|-----------|-------------|
| up | 0x48 | 0x26 (38) |
| down | 0x50 | 0x28 (40) |
| left | 0x4B | 0x25 (37) |
| right | 0x4D | 0x27 (39) |

### Block Navigation

| Name | Scan Code | Virtual Key |
|------|-----------|-------------|
| home | 0x47 | 0x24 (36) |
| end | 0x4F | 0x23 (35) |
| pageup | 0x49 | 0x21 (33) |
| pagedown | 0x51 | 0x22 (34) |

### Editing Keys

| Name | Alias(es) | Scan Code | Virtual Key |
|------|-----------|-----------|-------------|
| insert | ins | 0x52 | 0x2D (45) |
| delete | del, forwarddelete, forwarddel | 0x53 | 0x2E (46) |

**Vim-style navigation example:**
```toml
[keyboard.layers.vim]
activation_key = "CapsLock"
mode = "Hold"

[keyboard.layers.vim.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"
W = "Ctrl+Right"     # Next word
B = "Ctrl+Left"      # Previous word
U = "PageUp"
D = "PageDown"
X = "Delete"
```

---

## Special Keys

| Name | Alias(es) | Scan Code | Virtual Key | Description |
|------|-----------|-----------|-------------|-------------|
| capslock | caps | 0x3A | 0x14 (20) | Caps Lock |
| grave | backtick | 0x29 | 0xC0 (96) | Backtick / Grave accent |

The **grave** key is notable because `Alt+Grave` (`) is commonly bound to `SwitchToNextWindow`:

```toml
[window.shortcuts]
"Alt+Grave" = "SwitchToNextWindow"
```

---

## Punctuation & Symbol Keys (US Layout)

These keys are located on the main keyboard area and produce punctuation/symbol characters.

> **Note:** The actual character produced depends on keyboard layout and modifier state.
> These names refer to the physical key position (scan code).

| Name | Alias(es) | Scan Code | Virtual Key | US Layout (unshifted) | US Layout (shifted) |
|------|-----------|-----------|-------------|---------------------|--------------------|
| comma | `,` | 0x33 | 0xBC (188) | `,` | `<` |
| period | `.` | 0x34 | 0xBE (190) | `.` | `>` |
| semicolon | `;` | 0x27 | 0xBA (186) | `;` | `:` |
| quote | `'`, apostrophe | 0x28 | 0xDE (222) | `'` | `"` |
| bracketleft | `[` | 0x1A | 0xDB (219) | `[` | `{` |
| bracketright | `]` | 0x1B | 0xDD (221) | `]` | `}` |
| backslash | `\` | 0x2B | 0xDC (220) | `\` | `\|` |
| minus | `-` | 0x0C | 0xBD (189) | `-` | `_` |
| equal | `=` | 0x0D | 0xBB (187) | `=` | `+` |

**Example - numpad layer using symbol keys:**
```toml
[keyboard.layers.numpad]
activation_key = "RightAlt"
mode = "Hold"

[keyboard.layers.numpad.mappings]
M = "Numpad1"
Comma = "Numpad2"
Period = "Numpad3"
```

---

## Numpad Keys

The numeric keypad (numpad) has its own set of key names. All numpad keys support both `numpadX` and `numX` aliases.

### Numpad Digits

| Name | Alias(es) | Scan Code | Virtual Key |
|------|-----------|-----------|-------------|
| numpad0 | num0 | 0x52 | 0x60 (96) |
| numpad1 | num1 | 0x4F | 0x61 (97) |
| numpad2 | num2 | 0x50 | 0x62 (98) |
| numpad3 | num3 | 0x51 | 0x63 (99) |
| numpad4 | num4 | 0x4B | 0x64 (100) |
| numpad5 | num5 | 0x4C | 0x65 (101) |
| numpad6 | num6 | 0x4D | 0x66 (102) |
| numpad7 | num7 | 0x47 | 0x67 (103) |
| numpad8 | num8 | 0x48 | 0x68 (104) |
| numpad9 | num9 | 0x49 | 0x69 (105) |

### Numpad Operators

| Name | Alias(es) | Scan Code | Virtual Key | Description |
|------|-----------|-----------|-------------|-------------|
| numpaddot | numdot, numpaddecimal | 0x53 | 0x6E (110) | Decimal point |
| numpadenter | numenter | 0x1C | 0x0C (12) | Enter (numpad) |
| numpadadd | numplus | 0x4E | 0x6B (107) | Plus (+) |
| numpadsub | numminus | 0x4A | 0x6D (109) | Minus (-) |
| numpadmul | nummul, numpadmultiply | 0x37 | 0x6A (106) | Multiply (*) |
| numpaddiv | numslash, numpaddivide | 0x35 | 0x6F (111) | Divide (/) |

**Physical layout reference:**

```
 Numpad layout:
 ┌─────┬─────┬─────┐
 │  7  │  8  │  9  │   NumpadMul (*)
 ├─────┼─────┼─────┤
 │  4  │  5  │  6  │   NumpadSub (-)
 ├─────┼─────┼─────┤
 │  1  │  2  │  3  │   NumpadAdd (+)
 ├─────┼─────┼─────┤   ┌─────────┐
 │  0  │  .  │Ent│   │ NumpadDiv(/)│
 └─────┴─────┴─────┘   └─────────┘
```

---

## Key Name Resolution Order

When parsing a key name, the resolver checks sources in this order:

1. **keyboard-codes crate** (`keyboard_codes::Key::from_str()`) — supports standard cross-platform key names
2. **Legacy hardcoded mappings** in `parse_key()` — covers all keys listed in this document

If neither source recognizes the name, an error is returned: `"Unknown key name: <name>"`.

### Tips

- **Case-insensitive**: All key names match case-insensitively (`"F1"`, `"f1"`, `"F1"` all work)
- **Use descriptive names**: Prefer readable names like `"numpad7"` over numeric codes
- **Check with validation**: Run `wakem status` or check logs after editing config to catch typos early

---

For more configuration details, see [config.md](config.md).
For macro-related key information (scan codes for macro definitions), see [MACROS.md](MACROS.md).
