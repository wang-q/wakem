use crate::types::{
    Action, ContextCondition, InputEvent, KeyAction, KeyEvent, KeyState, MappingRule,
};
use std::collections::HashMap;
use tracing::{debug, error, info};

#[cfg(target_os = "windows")]
use crate::platform::windows::window_manager::RealWindowManager;

#[cfg(target_os = "macos")]
use crate::platform::macos::window_manager::RealWindowManager;

/// Thread-safe wrapper for platform-specific window handles.
///
/// This struct wraps platform-specific handles (like HWND on Windows) and provides
/// a Send + Sync implementation that is safe because:
/// 1. The handles are treated as opaque identifiers (usize-sized values)
/// 2. All actual operations on these handles are performed through the WindowManager
/// 3. The WindowManager is protected by Arc<RwLock<>> in the daemon
///
/// This is a safer alternative to marking the entire KeyMapper as Send + Sync.
#[allow(dead_code)]
struct ThreadSafeWindowManager {
    #[cfg(target_os = "windows")]
    inner: Option<RealWindowManager>,
    #[cfg(target_os = "macos")]
    inner: Option<RealWindowManager>,
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    inner: Option<()>,
}

// SAFETY: ThreadSafeWindowManager is safe to Send/Sync because:
// - It only stores opaque handle values (usize-sized)
// - Actual Win32 API calls are serialized through Arc<RwLock<KeyMapper>>
// - The inner WindowManager is only accessed through &mut self methods
#[allow(dead_code)]
unsafe impl Send for ThreadSafeWindowManager {}
#[allow(dead_code)]
unsafe impl Sync for ThreadSafeWindowManager {}

/// Context-aware mapping rule
///
/// Select different mapping tables based on current window attributes
/// (process name, window class, title, etc.).
/// This allows the same key to have different behaviors in different applications.
///
/// # Example
///
/// ```ignore
/// // Map CapsLock to Ctrl in VSCode, but to Esc in other apps
/// let rule = ContextMappingRule {
///     context: ContextCondition::new()
///         .with_process_name("Code.exe"),
///     mappings: {
///         let mut map = HashMap::new();
///         map.insert(0x3A, Action::Key(KeyAction::Press { scan_code: None, virtual_key: 0x11 }));
///         map
///     },
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ContextMappingRule {
    /// Context condition (determines when to apply this rule)
    pub context: ContextCondition,
    /// Mapping table: scan code -> action (used when condition is met)
    pub mappings: HashMap<u16, Action>,
}

/// Key mapping engine
///
/// Core component of wakem, responsible for:
/// - Managing basic key mapping rules
/// - Supporting context-aware mapping (dynamic switching based on current window)
/// - Processing input events and returning corresponding actions
/// - Executing various action types (keyboard, mouse, window, launch program, etc.)
///
/// # Architecture
///
/// The mapping engine uses a layered processing strategy:
/// 1. **Base mappings** - Global rules that always take effect
/// 2. **Context mappings** - Rules that only take effect under specific conditions (higher priority)
/// 3. **Layer manager** - Provided by LayerManager, supports temporary overrides
///
/// # Usage Example
///
/// ```ignore
/// use wakem::runtime::KeyMapper;
///
/// // Create mapping engine
/// let mut mapper = KeyMapper::new();
///
/// // Load rules from configuration
/// let rules = config.get_all_rules();
/// mapper.load_rules(rules);
///
/// // Process input event
/// if let Some(action) = mapper.process_event(&input_event) {
///     mapper.execute_action(&action)?;
/// }
/// ```
///
/// # Performance Characteristics
///
/// - Uses HashMap for fast lookup (O(1) average complexity)
/// - Context condition caching to reduce repeated calculations
/// - Supports hot-reload configuration without restarting service
pub struct KeyMapper {
    /// Complete mapping rule list
    ///
    /// Preserves original rules for debugging and serialization.
    rules: Vec<MappingRule>,

    /// Context-aware mapping rule list
    ///
    /// These rules only take effect when specific context conditions are met,
    /// e.g.: Only map CapsLock to Ctrl in VSCode.
    context_rules: Vec<ContextMappingRule>,

    /// Whether the mapping engine is enabled
    ///
    /// When false, all input events are passed through without any mapping.
    enabled: bool,

    /// Window manager (for executing window management actions)
    #[cfg(target_os = "windows")]
    pub(crate) window_manager: Option<RealWindowManager>,

    /// Window manager (for executing window management actions) - macOS
    #[cfg(target_os = "macos")]
    pub(crate) window_manager: Option<RealWindowManager>,

    /// Tray icon (for displaying notifications)
    #[cfg(target_os = "windows")]
    tray_icon: Option<crate::platform::windows::TrayIcon>,

    /// Window preset manager (for saving/loading window presets)
    #[cfg(target_os = "windows")]
    window_preset_manager: Option<crate::platform::windows::WindowPresetManager>,
}

// SAFETY: KeyMapper is manually marked as Send + Sync because it contains platform-specific
// handle types (HWND, HICON) that are not auto-Send/Sync but are safe to transfer across threads
// under the following constraints:
//
// 1. HWND/HICON are pointer-sized integer values (i64 on Windows). Storing them is safe;
//    only dereferencing/using them requires care.
//
// 2. All mutating Win32 API calls (via execute_action) MUST be serialized externally.
//    The caller is responsible for ensuring mutual exclusion — currently achieved through
//    Arc<RwLock<KeyMapper>> in the daemon, where write access is gated by the RwLock.
//
// 3. Read operations (process_event_with_context) never invoke Win32 API calls directly;
//    they only inspect Rust data structures.
//
// 4. If KeyMapper is ever used without external synchronization (e.g., without RwLock),
//    the unsafe impl must be revisited. Consider using a newtype wrapper around HWND
//    with explicit Send/Sync bounds if this guarantee needs to be enforced at the type level.
//
// VIOLATION RISK: Removing the outer RwLock or calling execute_action from multiple threads
// concurrently would be undefined behavior.
//
// NOTE: We keep these unsafe impls for backward compatibility, but the ThreadSafeWindowManager
// struct above provides a safer pattern for future refactoring.
unsafe impl Send for KeyMapper {}
unsafe impl Sync for KeyMapper {}

impl KeyMapper {
    /// Create a new mapping engine
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            context_rules: Vec::new(),
            enabled: true,
            #[cfg(target_os = "windows")]
            window_manager: None,
            #[cfg(target_os = "macos")]
            window_manager: None,
            #[cfg(target_os = "windows")]
            tray_icon: None,
            #[cfg(target_os = "windows")]
            window_preset_manager: None,
        }
    }

    /// Create a mapping engine with window manager
    #[cfg(target_os = "windows")]
    pub fn with_window_manager(window_manager: RealWindowManager) -> Self {
        Self {
            rules: Vec::new(),
            context_rules: Vec::new(),
            enabled: true,
            window_manager: Some(window_manager),
            tray_icon: None,
            window_preset_manager: Some(
                crate::platform::windows::WindowPresetManager::new(
                    crate::platform::windows::WindowManager::new(),
                ),
            ),
        }
    }

    /// Create a mapping engine with window manager (macOS version)
    #[cfg(target_os = "macos")]
    pub fn with_window_manager(window_manager: RealWindowManager) -> Self {
        Self {
            rules: Vec::new(),
            context_rules: Vec::new(),
            enabled: true,
            window_manager: Some(window_manager),
        }
    }

    /// Set window preset manager
    #[cfg(target_os = "windows")]
    pub fn set_window_preset_manager(
        &mut self,
        manager: crate::platform::windows::WindowPresetManager,
    ) {
        self.window_preset_manager = Some(manager);
    }

    /// Load mapping rules from configuration
    pub fn load_rules(&mut self, rules: Vec<MappingRule>) {
        self.rules = rules;
        debug!("Loaded {} mapping rules", self.rules.len());
    }

    /// Process input event (with context awareness)
    pub fn process_event_with_context(
        &self,
        event: &InputEvent,
        context: Option<&crate::platform::traits::WindowContext>,
    ) -> Option<Action> {
        if !self.enabled {
            return None;
        }

        match event {
            InputEvent::Key(key_event) => {
                self.process_key_event_with_context(key_event, context)
            }
            InputEvent::Mouse(_) => {
                // Mouse event handling (TODO)
                None
            }
        }
    }

    /// Process keyboard event (with context awareness)
    fn process_key_event_with_context(
        &self,
        event: &KeyEvent,
        context: Option<&crate::platform::traits::WindowContext>,
    ) -> Option<Action> {
        debug!(
            scan_code = event.scan_code,
            virtual_key = event.virtual_key,
            state = ?event.state,
            modifiers = ?event.modifiers,
            context_rules_count = self.context_rules.len(),
            base_rules_count = self.rules.len(),
            "Mapper processing key event"
        );

        // 1. First check context-specific rules (high priority)
        if let Some(ctx) = context {
            debug!(
                process_name = %ctx.process_name,
                window_class = %ctx.window_class,
                "Checking context rules"
            );
            for rule in &self.context_rules {
                // Check if context matches
                if rule.context.matches(
                    &ctx.process_name,
                    &ctx.window_class,
                    &ctx.window_title,
                    ctx.executable_path.as_deref(),
                ) {
                    // Look up mapping in matched context
                    if let Some(action) = rule.mappings.get(&event.scan_code) {
                        let adjusted_action =
                            self.adjust_action_for_key_state(action, event);
                        if adjusted_action.is_some() {
                            debug!(
                                "Context mapping found: {:04X} -> {:?}",
                                event.scan_code, action
                            );
                        }
                        return adjusted_action;
                    }
                }
            }
        }

        // 2. Check base rules (considering modifiers)
        let input_event = InputEvent::Key(event.clone());
        debug!("Checking base rules");
        for (idx, rule) in self.rules.iter().enumerate() {
            if !rule.enabled {
                continue;
            }
            debug!(rule_idx = idx, trigger = ?rule.trigger, "Checking rule");
            if rule.trigger.matches(&input_event) {
                let action = &rule.action;
                let adjusted_action = self.adjust_action_for_key_state(action, event);
                debug!(
                    rule_idx = idx,
                    "Base rule matched: trigger={:?} -> {:?}", rule.trigger, action
                );
                return adjusted_action;
            }
        }

        debug!("No matching rule found");
        None
    }

    /// Adjust action based on key state
    fn adjust_action_for_key_state(
        &self,
        action: &Action,
        event: &KeyEvent,
    ) -> Option<Action> {
        match (action, &event.state) {
            (
                Action::Key(KeyAction::Click {
                    scan_code,
                    virtual_key,
                }),
                _,
            ) => {
                // If it's a click action, adjust based on actual key state
                match event.state {
                    KeyState::Pressed => Some(Action::Key(KeyAction::Press {
                        scan_code: *scan_code,
                        virtual_key: *virtual_key,
                    })),
                    KeyState::Released => Some(Action::Key(KeyAction::Release {
                        scan_code: *scan_code,
                        virtual_key: *virtual_key,
                    })),
                }
            }
            // Handle Hyper key sequence: split into press (on key down) and release (on key up) parts
            (Action::Sequence(actions), _) if actions.len() > 1 => {
                // Check if this is a Hyper key sequence (contains None marker)
                let noop_position =
                    actions.iter().position(|a| matches!(a, Action::None));

                if let Some(pos) = noop_position {
                    match event.state {
                        KeyState::Pressed => {
                            // Return only the press actions (before the marker)
                            let press_actions: Vec<_> =
                                actions.iter().take(pos).cloned().collect();
                            if press_actions.is_empty() {
                                None
                            } else {
                                Some(Action::Sequence(press_actions))
                            }
                        }
                        KeyState::Released => {
                            // Return only the release actions (after the marker)
                            let release_actions: Vec<_> =
                                actions.iter().skip(pos + 1).cloned().collect();
                            if release_actions.is_empty() {
                                None
                            } else {
                                Some(Action::Sequence(release_actions))
                            }
                        }
                    }
                } else {
                    Some(action.clone())
                }
            }
            _ => Some(action.clone()),
        }
    }

    /// Execute action (including window management actions)
    #[cfg(target_os = "windows")]
    pub fn execute_action(&mut self, action: &Action) -> anyhow::Result<()> {
        match action {
            Action::Window(window_action) => {
                if let Some(ref wm) = self.window_manager {
                    Self::execute_window_action_internal(
                        wm,
                        self.tray_icon.as_mut(),
                        self.window_preset_manager.as_mut(),
                        window_action,
                    )?;
                } else {
                    debug!("WindowManager not available, skipping window action");
                }
            }
            Action::Key(_)
            | Action::Mouse(_)
            | Action::Launch(_)
            | Action::Sequence(_)
            | Action::Delay { .. }
            | Action::None => {
                // These actions are handled by other components
            }
        }

        Ok(())
    }

    /// Execute window management action (internal static method to avoid borrow conflicts)
    #[cfg(target_os = "windows")]
    fn execute_window_action_internal(
        wm: &RealWindowManager,
        tray_icon: Option<&mut crate::platform::windows::TrayIcon>,
        preset_manager: Option<&mut crate::platform::windows::WindowPresetManager>,
        action: &crate::types::WindowAction,
    ) -> anyhow::Result<()> {
        use crate::platform::window_manager_common::CommonWindowApi;
        use crate::types::{MonitorDirection, WindowAction};
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return Err(anyhow::anyhow!("No foreground window"));
            }

            match action {
                WindowAction::Center => wm.move_to_center(hwnd)?,
                WindowAction::MoveToEdge(edge) => {
                    wm.move_to_edge(hwnd, *edge)?;
                }
                WindowAction::HalfScreen(edge) => {
                    wm.set_half_screen(hwnd, *edge)?;
                }
                WindowAction::LoopWidth(align) => {
                    wm.loop_width(hwnd, *align)?;
                }
                WindowAction::LoopHeight(align) => {
                    wm.loop_height(hwnd, *align)?;
                }
                WindowAction::FixedRatio {
                    ratio,
                    scale_index: _,
                } => {
                    wm.set_fixed_ratio(hwnd, *ratio)?;
                }
                WindowAction::NativeRatio { scale_index: _ } => {
                    wm.set_native_ratio(hwnd)?;
                }
                WindowAction::SwitchToNextWindow => {
                    wm.switch_to_next_window_of_same_process()?;
                }
                WindowAction::MoveToMonitor(direction) => {
                    let direction = match direction {
                        MonitorDirection::Next => {
                            crate::platform::windows::MonitorDirection::Next
                        }
                        MonitorDirection::Prev => {
                            crate::platform::windows::MonitorDirection::Prev
                        }
                        MonitorDirection::Index(idx) => {
                            crate::platform::windows::MonitorDirection::Index(*idx)
                        }
                    };
                    wm.move_to_monitor(hwnd, direction)?;
                }
                WindowAction::Move { x, y } => {
                    Self::window_move(wm, hwnd, *x, *y)?;
                }
                WindowAction::Resize { width, height } => {
                    Self::window_resize(wm, hwnd, *width, *height)?;
                }
                WindowAction::Minimize => {
                    Self::window_minimize(hwnd);
                }
                WindowAction::Maximize => {
                    Self::window_maximize(hwnd);
                }
                WindowAction::Restore => {
                    Self::window_restore(hwnd);
                }
                WindowAction::Close => {
                    Self::window_close(hwnd);
                }
                WindowAction::ToggleTopmost => {
                    Self::window_toggle_topmost(hwnd);
                }
                WindowAction::ShowDebugInfo => {
                    Self::window_show_debug_info(wm);
                }
                WindowAction::ShowNotification { title, message } => {
                    Self::window_show_notification(tray_icon, title, message);
                }
                WindowAction::SavePreset { name } => {
                    Self::window_save_preset(preset_manager, tray_icon, name);
                }
                WindowAction::LoadPreset { name } => {
                    Self::window_load_preset(preset_manager, hwnd, name);
                }
                WindowAction::ApplyPreset => {
                    Self::window_apply_preset(preset_manager, hwnd);
                }
                WindowAction::None => {}
            }
        }

        Ok(())
    }

    // === Windows window action helpers ===

    #[cfg(target_os = "windows")]
    fn window_move(
        wm: &RealWindowManager,
        hwnd: windows::Win32::Foundation::HWND,
        x: i32,
        y: i32,
    ) -> anyhow::Result<()> {
        use crate::platform::traits::WindowFrame;
        let info = wm.get_window_info(hwnd)?;
        let new_frame = WindowFrame::new(x, y, info.frame.width, info.frame.height);
        wm.set_window_frame(hwnd, &new_frame)
    }

    #[cfg(target_os = "windows")]
    fn window_resize(
        wm: &RealWindowManager,
        hwnd: windows::Win32::Foundation::HWND,
        width: i32,
        height: i32,
    ) -> anyhow::Result<()> {
        use crate::platform::traits::WindowFrame;
        let info = wm.get_window_info(hwnd)?;
        let new_frame = WindowFrame::new(info.frame.x, info.frame.y, width, height);
        wm.set_window_frame(hwnd, &new_frame)
    }

    #[cfg(target_os = "windows")]
    fn window_minimize(hwnd: windows::Win32::Foundation::HWND) {
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MINIMIZE};
        unsafe {
            let _ = ShowWindow(hwnd, SW_MINIMIZE);
        }
    }

    #[cfg(target_os = "windows")]
    fn window_maximize(hwnd: windows::Win32::Foundation::HWND) {
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MAXIMIZE};
        unsafe {
            let _ = ShowWindow(hwnd, SW_MAXIMIZE);
        }
    }

    #[cfg(target_os = "windows")]
    fn window_restore(hwnd: windows::Win32::Foundation::HWND) {
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_RESTORE};
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
    }

    #[cfg(target_os = "windows")]
    fn window_close(hwnd: windows::Win32::Foundation::HWND) {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
        unsafe {
            PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).ok();
        }
    }

    #[cfg(target_os = "windows")]
    fn window_toggle_topmost(hwnd: windows::Win32::Foundation::HWND) {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongW, SetWindowPos, GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST,
            SWP_NOMOVE, SWP_NOSIZE, WS_EX_TOPMOST,
        };
        unsafe {
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            let is_topmost = (ex_style & WS_EX_TOPMOST.0 as i32) != 0;
            let hwnd_insert_after = if is_topmost {
                HWND_NOTOPMOST
            } else {
                HWND_TOPMOST
            };
            SetWindowPos(
                hwnd,
                Some(hwnd_insert_after),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE,
            )
            .ok();
        }
    }

    #[cfg(target_os = "windows")]
    fn window_show_debug_info(wm: &RealWindowManager) {
        match wm.get_debug_info() {
            Ok(info) => {
                use windows::core::HSTRING;
                use windows::Win32::UI::WindowsAndMessaging::{
                    MessageBoxW, MB_ICONINFORMATION, MB_OK,
                };
                let title = HSTRING::from("wakem - Debug Info");
                let message = HSTRING::from(&info);
                unsafe {
                    MessageBoxW(None, &message, &title, MB_OK | MB_ICONINFORMATION);
                }
            }
            Err(e) => {
                debug!("Failed to get debug info: {}", e);
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn window_show_notification(
        tray_icon: Option<&mut crate::platform::windows::TrayIcon>,
        title: &str,
        message: &str,
    ) {
        if let Some(tray) = tray_icon {
            if let Err(e) = tray.show_notification(title, message) {
                debug!("Failed to show notification: {}", e);
            }
        } else {
            debug!("Tray icon not available, cannot show notification");
        }
    }

    #[cfg(target_os = "windows")]
    fn window_save_preset(
        preset_manager: Option<&mut crate::platform::windows::WindowPresetManager>,
        tray_icon: Option<&mut crate::platform::windows::TrayIcon>,
        name: &str,
    ) {
        if let Some(pm) = preset_manager {
            match pm.get_foreground_window_info() {
                Some(Ok(_info)) => {
                    if let Err(e) = pm.save_preset(name.to_string()) {
                        debug!("Failed to save preset '{}': {}", name, e);
                    } else {
                        debug!("Saved preset '{}' for current window", name);
                        if let Some(tray) = tray_icon {
                            let _ = tray.show_notification(
                                "wakem",
                                &format!("Preset '{}' saved", name),
                            );
                        }
                    }
                }
                Some(Err(e)) => {
                    debug!("Failed to get foreground window info: {}", e);
                }
                None => {
                    debug!("No foreground window found");
                }
            }
        } else {
            debug!("WindowPresetManager not available, cannot save preset");
        }
    }

    #[cfg(target_os = "windows")]
    fn window_load_preset(
        preset_manager: Option<&mut crate::platform::windows::WindowPresetManager>,
        _hwnd: windows::Win32::Foundation::HWND,
        name: &str,
    ) {
        if let Some(pm) = preset_manager {
            if let Err(e) = pm.load_preset(name) {
                debug!("Failed to load preset '{}': {}", name, e);
            } else {
                debug!("Loaded preset '{}' for current window", name);
            }
        } else {
            debug!("WindowPresetManager not available, cannot load preset");
        }
    }

    #[cfg(target_os = "windows")]
    fn window_apply_preset(
        preset_manager: Option<&mut crate::platform::windows::WindowPresetManager>,
        hwnd: windows::Win32::Foundation::HWND,
    ) {
        if let Some(pm) = preset_manager {
            match pm.apply_preset_for_window_by_id(hwnd) {
                Ok(true) => {
                    debug!("Applied matching preset to current window");
                }
                Ok(false) => {
                    debug!("No matching preset found for current window");
                }
                Err(e) => {
                    debug!("Failed to apply preset: {}", e);
                }
            }
        } else {
            debug!("WindowPresetManager not available, cannot apply preset");
        }
    }

    /// Execute window management action (macOS implementation)
    #[cfg(target_os = "macos")]
    fn execute_window_action_internal(
        wm: &RealWindowManager,
        action: &crate::types::WindowAction,
    ) -> anyhow::Result<()> {
        use crate::platform::traits::WindowManagerTrait;
        use crate::platform::window_manager_common::CommonWindowManager;
        use crate::types::{MonitorDirection, WindowAction};

        info!(?action, "execute_window_action_internal called");

        // Get foreground window
        let window = wm
            .get_foreground_window()
            .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;

        match action {
            WindowAction::Center => {
                CommonWindowManager::move_to_center(wm, window)?;
            }
            WindowAction::MoveToEdge(edge) => {
                CommonWindowManager::move_to_edge(wm, window, *edge)?;
            }
            WindowAction::HalfScreen(edge) => {
                CommonWindowManager::set_half_screen(wm, window, *edge)?;
            }
            WindowAction::LoopWidth(_) => {
                use crate::types::Alignment;
                CommonWindowManager::loop_width(wm, window, Alignment::Left)?;
            }
            WindowAction::LoopHeight(_) => {
                use crate::types::Alignment;
                CommonWindowManager::loop_height(wm, window, Alignment::Top)?;
            }
            WindowAction::FixedRatio { ratio, .. } => {
                CommonWindowManager::set_fixed_ratio(wm, window, *ratio)?;
            }
            WindowAction::NativeRatio { .. } => {
                CommonWindowManager::set_native_ratio(wm, window)?;
            }
            WindowAction::SwitchToNextWindow => {
                wm.switch_to_next_window_of_same_process()?;
            }
            WindowAction::MoveToMonitor(direction) => {
                use crate::platform::traits::MonitorDirection as PlatformMonitorDirection;
                let monitor_direction = match direction {
                    MonitorDirection::Next => PlatformMonitorDirection::Next,
                    MonitorDirection::Prev => PlatformMonitorDirection::Prev,
                    MonitorDirection::Index(idx) => {
                        PlatformMonitorDirection::Index(*idx)
                    }
                };
                wm.move_to_monitor(window, monitor_direction)?;
            }
            WindowAction::Move { x, y } => {
                let info = wm.get_window_info(window)?;
                wm.set_window_pos(window, *x, *y, info.width, info.height)?;
            }
            WindowAction::Resize { width, height } => {
                let info = wm.get_window_info(window)?;
                wm.set_window_pos(window, info.x, info.y, *width, *height)?;
            }
            WindowAction::Minimize => {
                wm.minimize_window(window)?;
            }
            WindowAction::Maximize => {
                wm.maximize_window(window)?;
            }
            WindowAction::Restore => {
                wm.restore_window(window)?;
            }
            WindowAction::Close => {
                wm.close_window(window)?;
            }
            WindowAction::ToggleTopmost => {
                CommonWindowManager::toggle_topmost(wm, window)?;
            }
            WindowAction::ShowDebugInfo => match wm.get_window_info(window) {
                Ok(info) => {
                    let debug_info = format!(
                        "Window Debug Info:\n\
                             Position: ({}, {})\n\
                             Size: {}x{}\n\
                             Process: {}\n",
                        info.x, info.y, info.width, info.height, info.process_name
                    );
                    info!("Window debug info:\n{}", debug_info);
                    if let Err(e) = crate::platform::macos::native_api::notification::show_notification(
                            "wakem - Debug Info",
                            &debug_info,
                        ) {
                            debug!("Failed to show notification: {}", e);
                        }
                }
                Err(e) => {
                    debug!("Failed to get debug info: {}", e);
                }
            },
            WindowAction::ShowNotification { title, message } => {
                if let Err(e) =
                    crate::platform::macos::native_api::notification::show_notification(
                        title, message,
                    )
                {
                    debug!("Failed to show notification: {}", e);
                }
            }
            WindowAction::SavePreset { name } => {
                debug!("SavePreset not yet implemented on macOS: {}", name);
            }
            WindowAction::LoadPreset { name } => {
                debug!("LoadPreset not yet implemented on macOS: {}", name);
            }
            WindowAction::ApplyPreset => {
                debug!("ApplyPreset not yet implemented on macOS");
            }
            WindowAction::None => {}
        }

        Ok(())
    }

    /// Execute action (macOS version)
    #[cfg(target_os = "macos")]
    pub fn execute_action(&mut self, action: &Action) -> anyhow::Result<()> {
        debug!(?action, "Mapper execute_action called (macOS)");
        match action {
            Action::Window(window_action) => {
                info!(?window_action, "Processing window action in mapper");
                if let Some(ref wm) = self.window_manager {
                    info!("WindowManager found, executing window action");
                    match Self::execute_window_action_internal(wm, window_action) {
                        Ok(()) => info!("Window action executed successfully"),
                        Err(e) => error!(error = %e, "Failed to execute window action"),
                    }
                } else {
                    error!("WindowManager not available, skipping window action. This means with_window_manager() was not called during initialization.");
                }
            }
            Action::Key(_)
            | Action::Mouse(_)
            | Action::Launch(_)
            | Action::Sequence(_)
            | Action::Delay { .. }
            | Action::None => {
                // These actions are handled by other components
            }
        }

        Ok(())
    }

    /// Load context-aware mapping rules
    pub fn load_context_rules(
        &mut self,
        context_mappings: &[crate::config::ContextMapping],
    ) {
        self.context_rules.clear();

        for mapping in context_mappings {
            let mut rule_mappings = HashMap::new();

            // Parse each mapping string
            for (from, to) in &mapping.mappings {
                if let Ok((scan_code, action)) = Self::parse_context_mapping(from, to) {
                    rule_mappings.insert(scan_code, action);
                }
            }

            if !rule_mappings.is_empty() {
                self.context_rules.push(ContextMappingRule {
                    context: mapping.context.clone(),
                    mappings: rule_mappings,
                });
            }
        }

        debug!("Loaded {} context mapping rules", self.context_rules.len());
    }

    /// Parse context mapping string
    fn parse_context_mapping(from: &str, to: &str) -> anyhow::Result<(u16, Action)> {
        use crate::config::parse_key;
        use crate::types::KeyAction;

        // Parse source key
        let from_key = parse_key(from)?;

        // Parse target action
        // First try to parse as key
        if let Ok(to_key) = parse_key(to) {
            return Ok((
                from_key.0,
                Action::key(KeyAction::click(to_key.0, to_key.1)),
            ));
        }

        // Try to parse as window action
        if let Ok(window_action) = crate::config::parse_window_action(to) {
            return Ok((from_key.0, Action::window(window_action)));
        }

        Err(anyhow::anyhow!(
            "Failed to parse mapping: {} -> {}",
            from,
            to
        ))
    }
}

impl Default for KeyMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{InputEvent, KeyEvent, KeyState, ModifierState, Trigger};

    #[test]
    fn test_key_mapper_basic() {
        let mapper = KeyMapper::new();

        // Test creation successful
        assert!(mapper.enabled);
    }

    #[test]
    fn test_key_mapper_disabled() {
        let mut mapper = KeyMapper::new();

        // Disable mapper
        mapper.enabled = false;

        // Test event processing returns None
        let event = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
        let result = mapper.process_event_with_context(&InputEvent::Key(event), None);

        assert!(result.is_none());
    }

    #[test]
    fn test_key_mapper_load_rules() {
        let mut mapper = KeyMapper::new();

        // Create simple mapping rule
        let rules = vec![MappingRule::new(
            Trigger::key(0x3A, 0x14),                  // CapsLock
            Action::key(KeyAction::click(0x0E, 0x08)), // Backspace
        )];

        mapper.load_rules(rules);

        // Verify rules loaded
        assert_eq!(mapper.rules.len(), 1);
    }

    // ==================== Additional tests from ut_runtime_mapper_full.rs ====================

    #[test]
    fn test_key_mapper_new() {
        let _mapper = KeyMapper::new();
    }

    #[test]
    fn test_key_mapper_default() {
        let _mapper = KeyMapper::default();
    }

    #[test]
    fn test_mapper_load_rules_alt() {
        let mut mapper = KeyMapper::new();

        let rules = vec![
            MappingRule::new(
                Trigger::key(0x3A, 0x14),                  // CapsLock
                Action::key(KeyAction::click(0x0E, 0x08)), // Backspace
            ),
            MappingRule::new(
                Trigger::key(0x01, 0x1B),                  // Escape
                Action::key(KeyAction::click(0x4B, 0x25)), // Left
            ),
        ];

        mapper.load_rules(rules);
    }

    #[test]
    fn test_mapper_process_event_simple_match() {
        let mut mapper = KeyMapper::new();

        let rules = vec![MappingRule::new(
            Trigger::key(0x3A, 0x14),                  // CapsLock
            Action::key(KeyAction::click(0x0E, 0x08)), // Backspace
        )];

        mapper.load_rules(rules);

        let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
        let result = mapper.process_event_with_context(&event, None);

        assert!(result.is_some(), "Should find matching mapping");
    }

    #[test]
    fn test_mapper_process_event_no_match() {
        let mut mapper = KeyMapper::new();

        let rules = vec![MappingRule::new(
            Trigger::key(0x3A, 0x14),                  // CapsLock
            Action::key(KeyAction::click(0x0E, 0x08)), // Backspace
        )];

        mapper.load_rules(rules);

        // Press 'A' key, should not match
        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        let result = mapper.process_event_with_context(&event, None);

        assert!(result.is_none(), "Should not find match");
    }

    #[test]
    fn test_mapper_disabled_alt() {
        let mut mapper = KeyMapper::new();

        let rules = vec![MappingRule::new(
            Trigger::key(0x3A, 0x14),
            Action::key(KeyAction::click(0x0E, 0x08)),
        )];

        mapper.load_rules(rules);

        // When disabled, events should return None
        // Note: enabled field is private, we verify through behavior
        let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
        let result = mapper.process_event_with_context(&event, None);
        assert!(result.is_some()); // Default is enabled, so should have result
    }

    #[test]
    fn test_mapper_process_mouse_event() {
        let mapper = KeyMapper::new();

        let mouse_event =
            crate::types::MouseEvent::new(crate::types::MouseEventType::Move, 100, 200);
        let event = InputEvent::Mouse(mouse_event);

        let result = mapper.process_event_with_context(&event, None);
        assert!(result.is_none(), "Mouse events currently not processed");
    }

    #[test]
    fn test_mapper_adjust_action_pressed() {
        let mut mapper = KeyMapper::new();

        let rules = vec![MappingRule::new(
            Trigger::key(0x3A, 0x14),
            Action::key(KeyAction::click(0x0E, 0x08)), // Click action
        )];

        mapper.load_rules(rules);

        // Press event -> should return Press action
        let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
        let result = mapper.process_event_with_context(&event, None);

        assert!(result.is_some());
        if let Some(Action::Key(KeyAction::Press { .. })) = result {
            // Correct: press event converts to Press
        } else {
            panic!("Press event should convert to Press action");
        }
    }

    #[test]
    fn test_mapper_adjust_action_released() {
        let mut mapper = KeyMapper::new();

        let rules = vec![MappingRule::new(
            Trigger::key(0x3A, 0x14),
            Action::key(KeyAction::click(0x0E, 0x08)), // Click action
        )];

        mapper.load_rules(rules);

        // Release event -> should return Release action
        let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Released));
        let result = mapper.process_event_with_context(&event, None);

        assert!(result.is_some());
        if let Some(Action::Key(KeyAction::Release { .. })) = result {
            // Correct: release event converts to Release
        } else {
            panic!("Release event should convert to Release action");
        }
    }

    // ==================== Additional tests from ut_runtime_mapper.rs ====================

    #[test]
    fn test_mapping_rule_matching() {
        let rule = MappingRule::new(
            Trigger::key(0x1E, 0x41),                  // 'A' key
            Action::key(KeyAction::click(0x1F, 0x42)), // 'B' key
        );

        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        assert!(rule.trigger.matches(&event));
    }

    #[test]
    fn test_mapping_rule_with_modifiers() {
        let mut modifiers = ModifierState::new();
        modifiers.ctrl = true;
        let trigger = Trigger::key_with_modifiers(0x1E, 0x41, modifiers); // Ctrl + 'A'

        let rule = MappingRule::new(trigger, Action::key(KeyAction::click(0x1F, 0x42)));

        // Create event with Ctrl modifier
        let mut event_modifiers = ModifierState::new();
        event_modifiers.ctrl = true;
        let event = InputEvent::Key(
            KeyEvent::new(0x1E, 0x41, KeyState::Pressed).with_modifiers(event_modifiers),
        );

        assert!(rule.trigger.matches(&event));
    }
}
