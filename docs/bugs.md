# Bug Records

This document records complex bugs and their analysis for future reference.

---

## Bug 001: CapsLock as Hyper Key Produces @ Symbol

**Date:** 2026-05-04
**Status:** Open
**Labels:** `windows`, `hyper-key`, `keyboard`, `input-hook`

### Issue Description

When using CapsLock as a Hyper key (remapped to `Ctrl+Alt+Win`), pressing and releasing CapsLock causes an `@` symbol to appear at the cursor position.

User reports:
- First press: produces `@`
- First release: produces `@`
- Subsequent presses/releases: produces one `@` each

### Root Cause Analysis

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

### Attempted Solutions (Not Working)

#### Attempt 1: Remove Raw Input Keyboard Handling
- **Approach**: Removed keyboard event handling from `handle_raw_input`, keeping only Low-Level Hook for keyboard
- **Result**: Failed - `@` symbol still appears
- **Code Changes**:
  - Modified `input.rs` to skip keyboard events in `handle_raw_input`
  - Only mouse events are processed through Raw Input now

#### Attempt 2: Implement Synchronous Event Blocking in Low-Level Hook
- **Approach**: Added global `BLOCKED_KEYS` registry to track all mapped keys, modified `keyboard_proc` to return `LRESULT(1)` for blocked keys
- **Result**: Failed - `@` symbol still appears
- **Code Changes**:
  - Added `BLOCKED_KEYS: RwLock<Option<HashSet<(u16, u16)>>>` static variable
  - Added `set_blocked_keys()` function to update blocked key set
  - Modified `keyboard_proc` to check if key is in blocked set before allowing propagation
  - Used `parking_lot::RwLock` for better cross-thread synchronization

#### Attempt 3: Collect All Mapped Keys
- **Approach**: Added `Config::get_blocked_keys()` to collect all keys with mappings (remap, layers, shortcuts, macros, launch)
- **Result**: Failed - `@` symbol still appears
- **Code Changes**:
  - Added `get_blocked_keys()` method in `config.rs`
  - Modified `daemon.rs` to call `set_blocked_keys()` when loading configuration
  - Added test `test_get_blocked_keys` to verify correct key collection

### Current Code State

The following changes have been made but the issue persists:

**Files Modified:**
- `input.rs`: 
  - Uses `parking_lot::RwLock` for `BLOCKED_KEYS` synchronization
  - `keyboard_proc` checks blocked keys and returns `LRESULT(1)` to block events
  - Raw Input only handles mouse events
  
- `config.rs`: 
  - Added `get_blocked_keys()` method
  - Added `test_get_blocked_keys` test
  
- `daemon.rs`: 
  - Calls `set_blocked_keys()` on config load

### Hypotheses for Why It's Not Working

1. **Hook Installation Timing**: The Low-Level Hook might be installed after Windows has already started processing the key
2. **Windows System-Level Handling**: CapsLock might be handled at a lower level in Windows before the Hook sees it
3. **Race Condition**: The blocked keys set might not be populated when the first key events arrive
4. **Hook Not Receiving Events**: The Hook might not be receiving the CapsLock events at all (though logs show it is)
5. **Scan Code Mismatch**: The scan code from the Hook might not match what's in the blocked set

### Debugging Suggestions for Next Attempt

1. Add verbose logging in `keyboard_proc` to confirm:
   - Hook is receiving CapsLock events
   - Scan code matches expected value (0x3A)
   - `should_block` is being evaluated correctly
   - `LRESULT(1)` is being returned

2. Verify `set_blocked_keys` is being called with correct keys before any key press

3. Test with a simple key remap (not Hyper key) to verify blocking works at all

4. Consider using Windows `BlockInput` API as a nuclear option

5. Check if Windows is generating the `@` from the OutputDevice's simulated input rather than the original key

### Related Code Locations

| File | Line | Description |
|------|------|-------------|
| `input.rs` | 18-20 | `BLOCKED_KEYS` static variable |
| `input.rs` | 179-248 | `keyboard_proc` - Low-Level Hook with blocking logic |
| `input.rs` | 251-258 | `set_blocked_keys()` function |
| `config.rs` | 283-328 | `get_blocked_keys()` - collects all mapped keys |
| `daemon.rs` | 175-180 | Calls `set_blocked_keys()` on config load |

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
