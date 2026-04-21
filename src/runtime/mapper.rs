use crate::types::{
    Action, ContextCondition, InputEvent, KeyAction, KeyEvent, KeyState, MappingRule,
    Trigger,
};
use std::collections::HashMap;
use tracing::{debug, trace};

#[cfg(target_os = "windows")]
use crate::platform::windows::window_manager::RealWindowManager;

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
    /// Base mapping table: scan code -> action
    ///
    /// Stores global key mapping rules that always take effect.
    /// Key is scan code (hardware-related), value is the action to execute.
    mappings: HashMap<u16, Action>,

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
    window_manager: Option<RealWindowManager>,

    /// Tray icon (for displaying notifications)
    #[cfg(target_os = "windows")]
    tray_icon: Option<crate::platform::windows::TrayIcon>,

    /// Window preset manager (for saving/loading window presets)
    #[cfg(target_os = "windows")]
    window_preset_manager: Option<crate::platform::windows::WindowPresetManager>,
}

impl KeyMapper {
    /// Create a new mapping engine
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
            rules: Vec::new(),
            context_rules: Vec::new(),
            enabled: true,
            #[cfg(target_os = "windows")]
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
            mappings: HashMap::new(),
            rules: Vec::new(),
            context_rules: Vec::new(),
            enabled: true,
            window_manager: Some(window_manager),
            tray_icon: None,
            window_preset_manager: Some(
                crate::platform::windows::WindowPresetManager::new(),
            ),
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
        self.rebuild_mappings();
        debug!("Loaded {} mapping rules", self.rules.len());
    }

    /// Process input event (with context awareness)
    pub fn process_event_with_context(
        &self,
        event: &InputEvent,
        context: Option<&crate::platform::windows::WindowContext>,
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
        context: Option<&crate::platform::windows::WindowContext>,
    ) -> Option<Action> {
        trace!(
            "Processing key event: scan_code={:04X}, vk={:04X}, state={:?}",
            event.scan_code,
            event.virtual_key,
            event.state
        );

        // 1. First check context-specific rules (high priority)
        if let Some(ctx) = context {
            for rule in &self.context_rules {
                // Check if context matches
                if rule.context.matches(
                    &ctx.process_name,
                    &ctx.window_class,
                    &ctx.window_title,
                    Some(&ctx.executable_path),
                ) {
                    // Look up mapping in matched context
                    if let Some(action) = rule.mappings.get(&event.scan_code) {
                        let adjusted_action =
                            self.adjust_action_for_key_state(action, event);
                        if adjusted_action.is_some() {
                            trace!(
                                "Context mapping found: {:04X} -> {:?} (context: {:?})",
                                event.scan_code,
                                action,
                                rule.context
                            );
                        }
                        return adjusted_action;
                    }
                }
            }
        }

        // 2. Check base rules (considering modifiers)
        let input_event = InputEvent::Key(event.clone());
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            if rule.trigger.matches(&input_event) {
                let action = &rule.action;
                let adjusted_action = self.adjust_action_for_key_state(action, event);
                if adjusted_action.is_some() {
                    trace!(
                        "Base rule matched: trigger={:?} -> {:?}",
                        rule.trigger,
                        action
                    );
                }
                return adjusted_action;
            }
        }

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
            | Action::System(_)
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
        use crate::types::{MonitorDirection, WindowAction};
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0 == 0 {
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
                WindowAction::FixedRatio { ratio, scale_index } => {
                    wm.set_fixed_ratio(hwnd, *ratio, *scale_index)?;
                }
                WindowAction::NativeRatio { scale_index } => {
                    wm.set_native_ratio(hwnd, *scale_index)?;
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
                    use crate::platform::windows::WindowFrame;
                    let info = wm.get_window_info(hwnd)?;
                    let new_frame =
                        WindowFrame::new(*x, *y, info.frame.width, info.frame.height);
                    wm.set_window_frame(hwnd, &new_frame)?;
                }
                WindowAction::Resize { width, height } => {
                    use crate::platform::windows::WindowFrame;
                    let info = wm.get_window_info(hwnd)?;
                    let new_frame =
                        WindowFrame::new(info.frame.x, info.frame.y, *width, *height);
                    wm.set_window_frame(hwnd, &new_frame)?;
                }
                WindowAction::Minimize => {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        ShowWindow, SW_MINIMIZE,
                    };
                    let _ = ShowWindow(hwnd, SW_MINIMIZE);
                }
                WindowAction::Maximize => {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        ShowWindow, SW_MAXIMIZE,
                    };
                    let _ = ShowWindow(hwnd, SW_MAXIMIZE);
                }
                WindowAction::Restore => {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        ShowWindow, SW_RESTORE,
                    };
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                }
                WindowAction::Close => {
                    use windows::Win32::Foundation::{LPARAM, WPARAM};
                    use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
                    use windows::Win32::UI::WindowsAndMessaging::WM_CLOSE;
                    PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0)).ok();
                }
                WindowAction::ToggleTopmost => {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        GetWindowLongW, SetWindowPos, GWL_EXSTYLE, HWND_NOTOPMOST,
                        HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, WS_EX_TOPMOST,
                    };

                    // Correctly determine: check WS_EX_TOPMOST extended window style
                    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                    let is_topmost = (ex_style & WS_EX_TOPMOST.0 as i32) != 0;

                    let hwnd_insert_after = if is_topmost {
                        HWND_NOTOPMOST
                    } else {
                        HWND_TOPMOST
                    };

                    SetWindowPos(
                        hwnd,
                        hwnd_insert_after,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE,
                    )
                    .ok();
                }
                WindowAction::SetOpacity { opacity } => {
                    use windows::Win32::Foundation::COLORREF;
                    use windows::Win32::UI::WindowsAndMessaging::{
                        GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW,
                        GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED,
                    };

                    // Get current extended style
                    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);

                    // Add WS_EX_LAYERED style
                    if ex_style & WS_EX_LAYERED.0 as i32 == 0 {
                        SetWindowLongW(
                            hwnd,
                            GWL_EXSTYLE,
                            ex_style | WS_EX_LAYERED.0 as i32,
                        );
                    }

                    // Set transparency
                    SetLayeredWindowAttributes(hwnd, COLORREF(0), *opacity, LWA_ALPHA)
                        .ok();
                }
                WindowAction::ShowDebugInfo => {
                    // Show debug info dialog
                    match wm.get_debug_info() {
                        Ok(info) => {
                            use windows::core::HSTRING;
                            use windows::Win32::UI::WindowsAndMessaging::{
                                MessageBoxW, MB_ICONINFORMATION, MB_OK,
                            };

                            let title = HSTRING::from("wakem - Debug Info");
                            let message = HSTRING::from(&info);

                            MessageBoxW(
                                None,
                                &message,
                                &title,
                                MB_OK | MB_ICONINFORMATION,
                            );
                        }
                        Err(e) => {
                            debug!("Failed to get debug info: {}", e);
                        }
                    }
                }
                WindowAction::ShowNotification { title, message } => {
                    // Use tray icon to show notification
                    if let Some(tray) = tray_icon {
                        if let Err(e) = tray.show_notification(title, message) {
                            debug!("Failed to show notification: {}", e);
                        }
                    } else {
                        debug!("Tray icon not available, cannot show notification");
                    }
                }
                WindowAction::SavePreset { name } => {
                    if let Some(pm) = preset_manager {
                        match pm.get_foreground_window_info() {
                            Ok((hwnd, _title, process_name, executable_path)) => {
                                if let Err(e) = pm.save_preset(
                                    name,
                                    hwnd,
                                    process_name,
                                    executable_path,
                                    None, // Don't use title mode, use process name matching
                                ) {
                                    debug!("Failed to save preset '{}': {}", name, e);
                                } else {
                                    debug!("Saved preset '{}' for current window", name);
                                    // Show notification
                                    if let Some(tray) = tray_icon {
                                        let _ = tray.show_notification(
                                            "wakem",
                                            &format!("Preset '{}' saved", name),
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                debug!("Failed to get foreground window info: {}", e);
                            }
                        }
                    } else {
                        debug!("WindowPresetManager not available, cannot save preset");
                    }
                }
                WindowAction::LoadPreset { name } => {
                    if let Some(pm) = preset_manager {
                        if let Err(e) = pm.load_preset(name, hwnd) {
                            debug!("Failed to load preset '{}': {}", name, e);
                        } else {
                            debug!("Loaded preset '{}' for current window", name);
                        }
                    } else {
                        debug!("WindowPresetManager not available, cannot load preset");
                    }
                }
                WindowAction::ApplyPreset => {
                    if let Some(pm) = preset_manager {
                        match pm.apply_preset_for_window(hwnd) {
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
                WindowAction::None => {}
            }
        }

        Ok(())
    }

    /// Rebuild mapping table
    fn rebuild_mappings(&mut self) {
        self.mappings.clear();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            // Extract simple key mappings
            if let Trigger::Key {
                scan_code,
                virtual_key,
                ..
            } = &rule.trigger
            {
                if let Some(sc) = scan_code {
                    self.mappings.insert(*sc, rule.action.clone());
                } else if let Some(vk) = virtual_key {
                    // If no scan code, use virtual key as fallback
                    self.mappings.insert(*vk, rule.action.clone());
                }
            }
        }

        debug!("Rebuilt mappings: {} entries", self.mappings.len());
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
}
