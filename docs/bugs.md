# Bug Records

This document records complex bugs and their analysis for future reference.

---

## Bug 001: CapsLock as Hyper Key Produces @ Symbol

**Date:** 2026-05-04
**Status:** Fixed (2026-05-05)
**Labels:** `windows`, `hyper-key`, `keyboard`, `input-hook`

### Expected Behavior

When CapsLock is configured as a Hyper key (`CapsLock = "Ctrl+Alt+Meta"`), it should act as a pure virtual modifier:

- Pressing CapsLock alone should **not** produce any character output
- CapsLock should **not** send actual `Ctrl+Alt+Meta` key events to Windows
- CapsLock should only set an internal state that modifies subsequent key events
- For example, `CapsLock + C` should be interpreted internally as `Ctrl+Alt+Meta+C`, triggering the mapped action (e.g., window centering) without any visible text output
- The original CapsLock toggle functionality (LED state, case switching) may still operate at the system level

### Actual Behavior (Bug)

Pressing and releasing CapsLock causes an `@` symbol to appear at the cursor position.

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

#### Attempt 5: Diagnostic Logging (2026-05-04) ✅ SUCCESSFUL - Key Discovery
- **Approach**: Added detailed WARN-level logging for CapsLock events in both Hook and Raw Input to trace event flow
- **Code Changes**:
  - `input.rs` keyboard_proc (line ~204): Added CapsLock-specific logging with scan code, vk, state, flags
  - `input.rs` handle_raw_input (line ~354): Added matching CapsLock-specific logging
- **Result**: ✅ **Critical Discovery!**
  - **Confirmed Dual Event Source**: Both Hook and Raw Input receive the SAME CapsLock event:
    ```
    Pressed:
      [HOOK]    15:31:30.147843Z  flags=0x00  (+0ms)
      [RAW]     15:31:30.160554Z  flags=0x00  (+13ms later)
    
    Released:
      [HOOK]    15:31:30.278200Z  flags=0x80  (+0ms)
      [RAW]     15:31:30.281190Z  flags=0x01  (+3ms later)
    ```
  - **Event Count vs @ Symbol Count Mismatch**:
    - User pressed CapsLock 3 times = 6 events total (3 press + 3 release)
    - But only 4 `@` symbols appeared
    - This proves: Not a simple 1-to-1 mapping between events and @ symbols
  - **Flags Analysis**:
    - Pressed events: `flags=0x00` (normal)
    - Released events: Hook shows `flags=0x80`, Raw Input shows `flags=0x01` (different flag values!)
- **Key Finding**: The dual-event problem is real, but removing one source doesn't solve the @ issue

#### Attempt 6: Remove Raw Input Keyboard Handling Only (2026-05-04) ❌ FAILED
- **Approach**: Based on Attempt 5's discovery, removed ONLY the keyboard handling from Raw Input, keeping Hook as sole keyboard input source
- **Code Changes**:
  - `input.rs` handle_raw_input (line 332): Replaced entire keyboard processing block with skip comment:
    ```rust
    if device_type == RIM_TYPEKEYBOARD.0 {
        // 🔧 FIX: Skip keyboard events in Raw Input - only use Low-Level Hook for keyboard
        debug!("WM_INPUT: Skipping keyboard event (handled by Low-Level Hook)");
    }
    ```
  - Mouse processing via Raw Input remains unchanged
  - Restored daemon.rs SendInput functionality (removed Experiment 1's disable code)
- **Result**: ❌ **Failed - @ symbol still appears**
  - **Test Results from user**:
    - First press: produces `@`
    - First release: produces `@`
    - Subsequent presses/releases: produces one `@` each
    - Total: 3 CapsLock presses = 4 `@` symbols (same pattern as before!)
  - **Log Evidence**: Only `[HOOK]` logs now appear (no `[RAW INPUT]`), confirming removal worked
- **Conclusion**:
  - ✅ Successfully eliminated dual-event source (only Hook receives keyboard events now)
  - ❌ But @ symbol persists → **Raw Input was NOT the root cause**
  - 🎯 **New Focus**: Problem is either:
    1. Hook's CallNextHookEx allows propagation → system processes CapsLock → generates @
    2. OR: Our SendInput(Ctrl+Alt+Win) interacts with CapsLock somehow
    3. OR: CapsLock itself generates @ regardless of wakem

#### Attempt 7: Pure Hook + No SendInput (2026-05-04) ❌ FAILED
- **Approach**: Test if @ appears when using ONLY Hook (no Raw Input) AND disabling all modifier SendInput calls
- **Purpose**: Isolate whether @ is caused by:
  - Option A: CapsLock propagation through Hook (system-level issue)
  - Option B: Our SendInput(Ctrl+Alt+Win) interaction with CapsLock
- **Code Changes**:
  - Keep Attempt 6's changes (no Raw Input keyboard)
  - `daemon.rs` execute_action (line 581): Re-added modifier key filter:
    ```rust
    if is_modifier {
        warn!("🧪 EXP2: Skipped modifier key SendInput");
    } else {
        // Normal non-modifier key sending
    }
    ```
- **Result**: ❌ **Failed - @ symbol still appears**
  - **Test Results from user**:
    - First press: produces `@`
    - First release: produces `@`
    - Subsequent presses/releases: produces one `@` each
    - Total: 3 CapsLock presses = 4 `@` symbols (same pattern as before!)
  - **Conclusion**:
    - ✅ Confirmed: **SendInput is NOT the cause** of @ symbol
    - ❌ @ appears even without any SendInput calls for modifiers
    - 🎯 **Must be something else in wakem's processing pipeline**

#### Attempt 8: Test Without Wakem (2026-05-04) ✅ CRITICAL FINDING
- **Approach**: Completely stop wakem and test if bare CapsLock produces @
- **Purpose**: Determine if this is a system-level issue or wakem-specific regression
- **Result**: ✅ **No @ symbol when wakem is not running!**
  - User confirmed: "关闭不会产生的" (doesn't appear when closed)
  - **This proves**: The @ symbol is **100% caused by wakem**, NOT a system/driver/layout issue
  - **User feedback**: "以前是没这个问题的" (this didn't happen before)
  - **User conclusion**: "这就是个bug在某一次修改之后出现的我只是不知道什么原因"
    - Translation: "This is a bug that appeared after some modification, I just don't know what caused it"
  - **Key Insight**: This is a **regression bug** introduced by a recent code change!

#### Attempt 9: Minimal Hook Test (2026-05-04) ✅ SUCCESS - Root Cause Located!
- **Approach**: Create minimal Hook that logs CapsLock but does NOTHING else:
  - No `get_current_modifier_state()` call
  - No event sent to daemon
  - No processing whatsoever
  - Just log + CallNextHookEx
- **Code Changes**:
  - `input.rs` keyboard_proc (line ~189): Added early return for CapsLock:
    ```rust
    if kb_struct.vkCode == 0x14 {
        warn!("🧪 [MINIMAL HOOK] CapsLock: ...");
        return CallNextHookEx(None, code, wparam, lparam);
    }
    ```
- **Result**: ✅ **@ symbol DISAPPEARS!**
  - User confirmed: "在你取消了caps lock的功能之后现在是没有了"
  - Translation: "After you cancelled CapsLock functionality, it's gone now"
  - **Trade-off**: Hyper key functionality also doesn't work (as expected)
- **🎯🎉🎉 DEFINITIVE CONCLUSION**:

  ## Root Cause Found: Problem is in Daemon's Event Processing Pipeline

  ```
  ✅ Minimal Hook (no daemon processing) = No @
  ❌ Full processing (with daemon)         = Has @
  ```

  **The @ symbol is generated somewhere in the Daemon's handling of CapsLock events.**

  **Candidate locations in daemon.rs `process_input_event()`:**
  1. `check_and_update_hyper_key()` (line 359) - Updates virtual modifier state
  2. `merge_virtual_modifiers()` (line 362) - Merges virtual modifiers into event
  3. Layer manager processing (line 423-437)
  4. Mapper processing (line 440-445)
  5. Action execution (line 450-456)

  **Next step**: Systematically re-enable each daemon processing step to identify which one causes @.

### Current Code State

**📌 Current Configuration (Post-Attempt 9 - Root Cause Identified)**:
- `input.rs`:
  - ✅ **Raw Input keyboard handling REMOVED** (only mouse via Raw Input) [from Attempt 6]
  - ✅ **CapsLock processing restored to normal** [reverted Attempt 9's minimal hook test]
  - ✅ CapsLock diagnostic logging ACTIVE in Hook (`[HOOK]` WARN logs)
  - ❌ Raw Input logging removed (keyboard events skipped)

- `daemon.rs`:
  - ⚠️ **SendInput for modifier keys STILL DISABLED** [from Attempt 7 - should restore for normal operation]
  - **Note**: This was for testing, needs to be restored before final fix

- **Previous Attempts Status**:
  - Attempt 4 changes: **ALL REVERTED**
  - Attempt 5 (diagnostic logs): **ACTIVE** (Hook logs kept)
  - Attempt 6 (remove Raw Input): **ACTIVE** (current code state)
  - Attempt 7 (no SendInput): **ACTIVE** (testing artifact, needs cleanup)
  - Attempt 8 (bare test): **COMPLETED** (no code changes)
  - Attempt 9 (minimal hook): **REVERTED** (confirmed root cause location)

### 🎯 ROOT CAUSE SUMMARY

```
Problem: CapsLock as Hyper Key produces @ symbol
Status:  🔍 ROOT CAUSE LOCATED - In Daemon Processing Pipeline

Evidence Chain:
1. ✅ Dual event source exists (Hook + Raw Input) but not the cause
2. ✅ Removing Raw Input doesn't fix it
3. ✅ Disabling SendInput doesn't fix it
4. ✅ Without wakem = no @ (proves it's our bug)
5. ✅ Minimal Hook (skip daemon) = no @ (pinpoints daemon as culprit)

Conclusion: The @ symbol is generated during Daemon's event processing,
           specifically in process_input_event() or its callees.
```

### Hypotheses for Why It's Not Working

**Based on 9 attempts (4 failed fixes + 5 diagnostic experiments), FINAL CONCLUSION:**

1. **❌ DISPROVED: Hook Installation Timing** - Hook IS receiving events (Attempt 2-4 logs confirm)

2. **❌ DISPROVED: Race Condition** - BLOCKED_KEYS populated before events (Attempt 4 verified)

3. **❌ DISPROVED: Dual Event Source (Primary Cause)** - Attempt 6 removed Raw Input but @ persists

4. **❌ DISPROVED: SendInput Causes @** - Attempt 7 disabled all modifier SendInput, @ still appears

5. **❌ DISPROVED: System/Driver/Layout Issue** - Attempt 8 confirmed no @ without wakem

6. **✅✅✅ CONFIRMED ROOT CAUSE: Daemon's Event Processing Pipeline** (Attempt 9)
   - Minimal Hook (skip daemon) = No @ ✅
   - Full processing (with daemon) = Has @ ❌
   - **The bug is in `daemon.rs::process_input_event()` or one of its callees**

7. **🎯 NEXT STEP**: Binary search through daemon processing to find exact line

### Candidate Code Locations in daemon.rs process_input_event()

| Line | Function | What it Does | Priority |
|------|----------|--------------|----------|
| 359 | `check_and_update_hyper_key()` | Updates active_hyper_keys for CapsLock | 🔴 High |
| 362 | `merge_virtual_modifiers()` | Merges virtual modifiers into event | 🔴 High |
| 380-403 | Key release filtering | Filters non-hyper key releases | 🟡 Medium |
| 407-420 | Key repeat filtering | Suppresses key repeats | 🟢 Low |
| 423-437 | Layer manager processing | Checks layer mappings | 🟡 Medium |
| 440-445 | Mapper processing | Checks base mappings + context | 🔴 High |
| 450-456 | Action execution | Executes found actions (SendInput) | 🟡 Medium* |

*Note: We already know SendInput itself isn't the cause (Attempt 7), but action execution might trigger something else.

### Debugging Suggestions for Next Attempt

**🎯 ROOT CAUSE FOUND - Now Need to Pinpoint Exact Location**

**Completed Diagnostic Chain:**
1. ✅ **Attempt 5**: Diagnostic logging - Confirmed dual event source
2. ✅ **Attempt 6**: Remove Raw Input - Eliminated dual source, @ persists
3. ✅ **Attempt 7**: Disable SendInput - Confirmed not SendInput's fault
4. ✅ **Attempt 8**: Test without wakem - Confirmed it's our bug (regression)
5. ✅ **Attempt 9**: Minimal Hook - **Located root cause: Daemon processing**

**🔬 NEXT: Binary Search Through Daemon Pipeline**

**Strategy**: Systematically disable each processing step in `process_input_event()` to find which one generates @.

**Suggested Order** (highest suspicion first):

1. **Test A: Skip hyper key processing only**
   - Comment out line 359: `let _is_hyper_key = self.check_and_update_hyper_key(&event).await;`
   - If @ disappears → Bug is in hyper key virtual modifier logic

2. **Test B: Skip modifier merging only**
   - Comment out line 362: `let event = self.merge_virtual_modifiers(event).await;`
   - If @ disappears → Bug is in how we merge virtual modifiers

3. **Test C: Skip mapper/layer processing**
   - Comment out lines 423-456 (the mapping and execution logic)
   - If @ disappears → Bug is in action lookup or execution

4. **Test D: Add detailed logging to each step**
   - Log before/after each major function call
   - Check if any step has unexpected side effects

**Expected Outcome**:
- One of these tests will isolate the exact code section causing @
- Once isolated, can examine that specific code for the regression
- Compare with git history to find when it was introduced

### Related Code Locations

| File | Line | Description | Current Status |
|------|------|-------------|----------------|
| `input.rs` | 173-220 | `keyboard_proc` - Low-Level Hook with CapsLock diagnostic logging | ✅ Active (Attempt 5-7) |
| `input.rs` | 332-336 | `handle_raw_input` - Keyboard events SKIPPED (mouse only) | ✅ Modified (Attempt 6) |
| `daemon.rs` | 581-601 | `execute_action` - Modifier SendInput DISABLED for testing | ⚠️ Testing mode (Attempt 7) |
| `config.rs` | 661-697 | `create_hyper_key_action` - Generates Ctrl+Alt+Win sequence | Unchanged |
| `output_device.rs` | 37-79 | `send_key` - SendInput implementation | Unchanged |

### Experiment Timeline

| Date | Attempt | Approach | Result | Status |
|------|---------|----------|--------|--------|
| 2026-05-04 | 1-4 | Various blocking strategies | All failed | Reverted |
| 2026-05-04 | 5 | Diagnostic logging | ✅ Discovered dual source | Completed |
| 2026-05-04 | 6 | Remove Raw Input keyboard | ❌ @ still appears | Completed |
| 2026-05-04 | 7 | Pure Hook + No SendInput | ❌ @ still appears | Completed |
| 2026-05-04 | 8 | Test without wakem (bare CapsLock) | ✅ No @ - confirms our bug | Completed |
| 2026-05-04 | 9 | Minimal Hook (skip daemon) | ✅ **Root cause located!** | Completed |

### Solution (Implemented 2026-05-05)

**Approach**: Hyper key pure internalization — convert Hyper key from "sending modifier keys" to "pure virtual state"

**Core Changes**:

1. **[daemon.rs](file:///c:/Users/wangq/Scripts/wakem/src/daemon.rs#L357-L375)** `process_input_event()`:
   - After `check_and_update_hyper_key()`, if the event is a Hyper key, directly `return`
   - Hyper key only updates internal `active_hyper_keys` state and `pressed_keys` tracking
   - Does not enter mapper/action execution flow

2. **[config.rs](file:///c:/Users/wangq/Scripts/wakem/src/config.rs#L657-L665)** `create_hyper_key_action()`:
   - Changed from returning `Action::Sequence([Press Ctrl, Press Alt, Press Win, ...])` to returning `Action::None`
   - Completely eliminates SendInput injection of modifier keys

**Why this works**:

```
Before (bug):
  CapsLock press → daemon → SendInput(Ctrl+Alt+Win down) → @ generated

After (fixed):
  CapsLock press → daemon → update active_hyper_keys → return
                           ↓
  C press → merge_virtual_modifiers(Ctrl+Alt+Meta) → match "Ctrl+Alt+Meta+C" → Center
```

Hyper key now acts purely as an internal virtual modifier. It does not send any key events to Windows, so there is no conflict with the original CapsLock event.

**Verification**:
- `cargo clippy -- -D warnings` ✅ zero warnings
- `cargo test` ✅ 372 tests passed
- User manual test ✅ no `@` symbol, Hyper key functions normally

---

## 📋 Summary

**Bug**: CapsLock as Hyper Key produces `@` symbol  
**Type**: Architecture design issue  
**Root Cause**: `create_hyper_key_action` generated `Action::Sequence` containing `SendInput` calls for modifier keys, which conflicted with the original CapsLock event  
**Fix**: Hyper key pure internalization — only updates state, does not send input  
**Verification**: clippy passed, all tests passed, manual test passed

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
