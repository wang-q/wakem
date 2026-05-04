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

#### Attempt 4: Complete Event Blocking Architecture (2026-05-04) ❌ FAILED - REGRESSION
- **Approach**: Comprehensive fix combining all previous attempts with additional safeguards:
  1. Removed Raw Input keyboard handling (only mouse via Raw Input)
  2. Added `BLOCKED_KEYS: Lazy<RwLock<HashSet>>` global registry
  3. Enhanced `keyboard_proc` to:
     - Detect injected events via `LLKHF_INJECTED` (flags & 0x10)
     - Check blocked keys and return `LRESULT(1)` for mapped keys
     - Still send events to daemon even when blocking
  4. Marked all SendInput calls with `INJECTION_MARKER` (0xFFFE1234) in `dwExtraInfo`
  5. Implemented comprehensive `get_blocked_keys()` collecting:
     - Remap source keys (including hyper keys)
     - Layer activation keys + layer internal mappings
     - Window shortcut trigger keys
     - Launch trigger keys
  6. Called `set_blocked_keys()` in daemon's `load_config()`

- **Result**: ❌ **CRITICAL FAILURE** - Made things worse!
  - **Original bug persists**: CapsLock still produces `@` symbols
  - **New regression introduced**: Ctrl+C (and likely other Ctrl combinations) stopped working
  - User reported complete breakdown of normal keyboard functionality

- **Root Cause Analysis of Failure**:
  1. **Over-blocking issue**: `get_blocked_keys()` collected too many keys
     - When user configures `CapsLock = "Ctrl+Alt+Win"`, the method collected:
       - CapsLock as remap source ✓ (correct)
       - But may have also included modifier keys incorrectly
     - Ctrl key itself might have been added to BLOCKED_KEYS
     - Result: All Ctrl+X combinations were blocked from reaching the system
  
  2. **Hook blocking ineffective for CapsLock**:
     - Despite returning `LRESULT(1)`, Windows still processed CapsLock
     - Possible reasons:
       - CapsLock is a toggle key with special system-level handling
       - Keyboard driver processes it before hook
       - Hook cannot block toggle state changes

  3. **Timing/Architecture Issue**:
     - The @ symbol is generated BEFORE or INDEPENDENTLY of our hook
     - Likely generated by keyboard layout driver at lower level
     - Our blocking comes too late in the processing chain

- **Code Changes (ALL REVERTED)**:
  - `input.rs`: +102 lines (BLOCKED_KEYS, injection detection, blocking logic)
  - `output_device.rs`: 9 lines changed (INJECTION_MARKER)
  - `config.rs`: +66 lines (get_blocked_keys method)
  - `daemon.rs`: +10 lines (set_blocked_keys call)

- **Key Lesson Learned**:
  - Low-Level Hook blocking (`LRESULT(1)`) is **insufficient** for preventing CapsLock side effects
  - Over-aggressive key blocking breaks normal functionality
  - Need fundamentally different approach (not just blocking)

### Current Code State

**⚠️ ALL CHANGES REVERTED** - Code is back to pre-Attempt 4 state.

The following attempts were made but all failed:

**Attempted but Reverted Changes:**
- ~~`input.rs`~~: 
  - ~~Uses `once_cell::sync::Lazy<parking_lot::RwLock>` for `BLOCKED_KEYS`~~
  - ~~`keyboard_proc` checks LLKHF_INJECTED and blocked keys~~
  - ~~Returns `LRESULT(1)` for blocked keys~~
  - ~~Raw Input only handles mouse events~~
  
- ~~`output_device.rs`~~: 
  - ~~All SendInput use `INJECTION_MARKER` (0xFFFE1234)~~
  
- ~~`config.rs`~~: 
  - ~~Added `get_blocked_keys()` method~~
  
- ~~`daemon.rs`~~: 
  - ~~Calls `set_blocked_keys()` on config load~~

### Hypotheses for Why It's Not Working

**Based on 4 failed attempts, refined hypotheses:**

1. **❌ DISPROVED: Hook Installation Timing** - Hook IS receiving events (Attempt 2-4 logs confirm)

2. **❌ DISPROVED: Race Condition** - BLOCKED_KEYS populated before events (Attempt 4 verified)

3. **✅ LIKELY: Windows System-Level Handling** - CapsLock has special handling:
   - CapsLock is a **toggle key** with state maintained by keyboard driver
   - The toggle state change may happen BEFORE hook processes the event
   - `LRESULT(1)` may prevent key repeat but not initial state change
   - Keyboard layout driver (ToAscii/ToUnicode) may run at kernel level

4. **✅ LIKELY: @ Symbol Generated at Lower Level**:
   - The @ symbol is likely generated by keyboard layout driver
   - This happens when Ctrl+Alt (AltGr) state is combined with scan code
   - Our SendInput(Ctrl+Alt+Win) sets modifier state BEFORE original CapsLock is processed
   - Or: Original CapsLock reaches layout driver while our modifiers are already active

5. **⚠️ NEW: Over-blocking Danger** (from Attempt 4):
   - Collecting "all mapped keys" is too aggressive
   - Cannot block modifier keys (Ctrl/Alt/Shift/Win) that are used in combinations
   - Blocking strategy must be extremely selective

6. **🔬 FUNDAMENTAL QUESTION**: Where exactly is @ generated?
   - Option A: Original physical CapsLock event + our simulated modifiers
   - Option B: Our SendInput(Ctrl+Alt) somehow triggers it
   - Option C: Timing issue where both arrive at layout driver simultaneously

### Debugging Suggestions for Next Attempt

**From Attempt 4 failure, revised approach needed:**

1. ❌ **ABANDONED**: Global blocking of all mapped keys (breaks normal functionality)
   
2. ✅ **NEW DIRECTION**: Investigate exact source of @ symbol
   - Add logging to track EXACT sequence of events when CapsLock pressed
   - Log timing: When does SendInput execute vs when does @ appear?
   - Test: Disable SendInput completely, see if @ still appears
   - Test: Don't set any modifiers, just block CapsLock, see if @ appears

3. ✅ **NEW DIRECTION**: Selective blocking only for specific keys
   - Only block the hyper key trigger (CapsLock), NOT its target modifiers
   - Never add Ctrl/Alt/Shift/Win to BLOCKED_KEYS
   - Be extremely conservative about what gets blocked

4. ⚠️ **NUCLEAR OPTION** (high risk): Use low-level keyboard filter driver
   - Requires kernel-mode code
   - Can truly prevent key from reaching user mode
   - Complex deployment and signing requirements

5. 🔬 **RESEARCH NEEDED**: How do other tools solve this?
   - AutoHotkey: How does it handle CapsLock remapping?
   - PowerToys Keyboard Manager: Same scenario?
   - KBL: Kernel-level filtering?

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
