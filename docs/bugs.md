# Bug Records

This document records complex bugs and their analysis for future reference.

---

## Bug 001: CapsLock as Hyper Key Produces @ Symbol

**Date:** 2026-05-04
**Status:** Analyzing
**Labels:** `windows`, `hyper-key`, `keyboard`, `input-hook`

### Issue Description

When using CapsLock as a Hyper key (remapped to `Ctrl+Alt+Win`), pressing and releasing CapsLock causes an `@` symbol to appear at the cursor position.

User reports:
- First press: produces `@`
- First release: produces `@`
- Subsequent presses/releases: produces one `@` each

### Root Cause Analysis (In Progress)

#### Current Findings

1. **Dual Input Mechanisms**: The codebase uses both Raw Input and Low-Level Keyboard Hook simultaneously:
   - Raw Input (`handle_raw_input`): Receives `WM_INPUT` messages, **cannot block** events
   - Low-Level Hook (`keyboard_proc`): Global hook, **can block** events by returning non-zero

2. **Event Flow**:
   ```
   CapsLock pressed
     ↓
   Raw Input captures → sends to daemon → original event propagates to system
     ↓
   Low-Level Hook captures → sends to daemon → can be blocked
   ```

3. **The Problem**: Raw Input cannot block the original CapsLock event from reaching the system. Even if the Low-Level Hook blocks it, Raw Input has already allowed the event to propagate.

4. **Why @ Symbol?**: When CapsLock (scan code 0x3A) reaches the system while Hyper modifiers (Ctrl+Alt) are active:
   - Windows interprets `Ctrl+Alt` as `AltGr`
   - In many keyboard layouts, `AltGr + 0x3A` maps to `@`

### Failed Attempts

1. **Attempt 1**: Block in Low-Level Hook only
   - Result: Failed - Raw Input still propagates the event

### Proposed Solution

Remove Raw Input keyboard handling, use only Low-Level Keyboard Hook for keyboard events.

**Rationale**:
- Low-Level Hook can truly block events
- Raw Input is better suited for mouse/tablet input where blocking isn't needed
- Having both causes duplicate events and blocking failures

### Implementation Plan

1. Remove keyboard handling from `handle_raw_input` in `input.rs`
2. Keep only Low-Level Hook for keyboard events
3. Keep Raw Input for mouse events (if needed)

### Related Code Locations

| File | Line | Description |
|------|------|-------------|
| `input.rs` | 336-396 | Raw Input keyboard handling |
| `input.rs` | 189-260 | Low-Level Keyboard Hook |
| `config.rs` | 258-280 | `get_hyper_key_mappings()` |
| `daemon.rs` | 380-403 | Hyper key release filtering |

### Testing Notes

To reproduce:
1. Configure `CapsLock = "Ctrl+Alt+Win"` in config
2. Start wakem
3. Press CapsLock
4. Release CapsLock
5. Observe `@` symbols appearing

Expected after fix:
- No `@` symbol should appear
- Hyper key functionality should work normally

---

## Template for New Bugs

**Date:** YYYY-MM-DD
**Status:** [Open/Analyzing/Fixed/Closed]
**Labels:** `label1`, `label2`

### Issue Description

[Describe the bug]

### Root Cause Analysis

[Analysis details]

### Solution

[Proposed or implemented solution]

### Related Code Locations

| File | Line | Description |
|------|------|-------------|
| `file.rs` | 123 | Description |

### Testing Notes

[How to reproduce and verify]

### References

[Links and resources]

---
