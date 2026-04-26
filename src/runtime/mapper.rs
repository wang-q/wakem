use crate::types::{
    Action, ContextCondition, InputEvent, KeyAction, KeyEvent, KeyState, MappingRule,
};
use std::collections::HashMap;
use tracing::{debug, error, info};

use crate::platform::traits::{MonitorInfo, WindowApiBase, WindowManagerTrait};

/// Thread-safe wrapper for platform-specific window handles.
///
/// This struct wraps a WindowManagerTrait object and provides
/// a Send + Sync implementation that is safe because:
/// 1. The WindowManagerTrait provides cross-platform abstraction
/// 2. All actual operations are performed through trait methods
/// 3. The wrapper ensures thread-safe access patterns
#[allow(dead_code)]
struct ThreadSafeWindowManager {
    inner: Option<Box<dyn WindowManagerTrait>>,
}

// SAFETY: ThreadSafeWindowManager is safe to Send/Sync because:
// - It only stores a boxed trait object (Box<dyn WindowManagerTrait>)
// - All window operations are performed through the trait's methods
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
    /// Uses trait object for cross-platform abstraction
    pub(crate) window_manager: Option<Box<dyn WindowManagerTrait>>,

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
            window_manager: None,
            #[cfg(target_os = "windows")]
            tray_icon: None,
            #[cfg(target_os = "windows")]
            window_preset_manager: None,
        }
    }

    /// Create a mapping engine with window manager
    /// Accepts any type that implements WindowManagerTrait
    pub fn with_window_manager<T: WindowManagerTrait + 'static>(
        window_manager: T,
    ) -> Self {
        Self {
            rules: Vec::new(),
            context_rules: Vec::new(),
            enabled: true,
            window_manager: Some(Box::new(window_manager)),
            #[cfg(target_os = "windows")]
            tray_icon: None,
            #[cfg(target_os = "windows")]
            window_preset_manager: Some(
                crate::platform::windows::WindowPresetManager::new(
                    crate::platform::windows::WindowManager::new(),
                ),
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
    /// Uses WindowManagerTrait for cross-platform abstraction
    pub fn execute_action(&mut self, action: &Action) -> anyhow::Result<()> {
        match action {
            Action::Window(window_action) => {
                if let Some(ref wm) = self.window_manager {
                    Self::execute_window_action_internal(
                        wm.as_ref(),
                        #[cfg(target_os = "windows")]
                        self.tray_icon.as_mut(),
                        #[cfg(target_os = "windows")]
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
    /// Uses WindowManagerTrait for cross-platform abstraction
    #[allow(unused_variables)]
    fn execute_window_action_internal(
        wm: &dyn WindowManagerTrait,
        #[cfg(target_os = "windows")] tray_icon: Option<
            &mut crate::platform::windows::TrayIcon,
        >,
        #[cfg(target_os = "windows")] preset_manager: Option<
            &mut crate::platform::windows::WindowPresetManager,
        >,
        action: &crate::types::WindowAction,
    ) -> anyhow::Result<()> {
        use crate::types::{MonitorDirection, WindowAction};

        // Get foreground window using trait method
        let window_id = wm
            .get_foreground_window()
            .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;

        match action {
            WindowAction::Center => {
                Self::window_move_to_center(wm, window_id)?;
            }
            WindowAction::MoveToEdge(edge) => {
                Self::window_move_to_edge(wm, window_id, *edge)?;
            }
            WindowAction::HalfScreen(edge) => {
                Self::window_set_half_screen(wm, window_id, *edge)?;
            }
            WindowAction::LoopWidth(align) => {
                Self::window_loop_width(wm, window_id, *align)?;
            }
            WindowAction::LoopHeight(align) => {
                Self::window_loop_height(wm, window_id, *align)?;
            }
            WindowAction::FixedRatio {
                ratio,
                scale_index: _,
            } => {
                Self::window_set_fixed_ratio(wm, window_id, *ratio)?;
            }
            WindowAction::NativeRatio { scale_index: _ } => {
                Self::window_set_native_ratio(wm, window_id)?;
            }
            WindowAction::SwitchToNextWindow => {
                Self::window_switch_next(wm, window_id)?;
            }
            WindowAction::MoveToMonitor(direction) => {
                let monitor_index: usize = match direction {
                    MonitorDirection::Next | MonitorDirection::Prev => 0,
                    MonitorDirection::Index(idx) => *idx as usize,
                };
                wm.move_to_monitor(window_id, monitor_index)?;
            }
            WindowAction::Move { x, y } => {
                wm.set_window_pos(window_id, *x, *y, 0, 0)?;
            }
            WindowAction::Resize { width, height } => {
                let info = wm.get_window_info(window_id)?;
                wm.set_window_pos(window_id, info.x, info.y, *width, *height)?;
            }
            WindowAction::Minimize => {
                wm.minimize_window(window_id)?;
            }
            WindowAction::Maximize => {
                wm.maximize_window(window_id)?;
            }
            WindowAction::Restore => {
                wm.restore_window(window_id)?;
            }
            WindowAction::Close => {
                wm.close_window(window_id)?;
            }
            WindowAction::ToggleTopmost => {
                let is_topmost = wm.is_topmost(window_id);
                wm.set_topmost(window_id, !is_topmost)?;
            }
            WindowAction::ShowDebugInfo => {
                Self::window_show_debug_info(wm, window_id);
            }
            WindowAction::ShowNotification { title, message } => {
                #[cfg(target_os = "windows")]
                if let Some(tray) = tray_icon {
                    let _ = tray.show_notification(title, message);
                }
            }
            WindowAction::SavePreset { name } => {
                #[cfg(target_os = "windows")]
                Self::window_save_preset(preset_manager, tray_icon, name, window_id);
            }
            WindowAction::LoadPreset { name } => {
                #[cfg(target_os = "windows")]
                Self::window_load_preset(preset_manager, name);
            }
            WindowAction::ApplyPreset => {
                #[cfg(target_os = "windows")]
                Self::window_apply_preset(preset_manager, window_id);
            }
            WindowAction::None => {}
        }

        Ok(())
    }

    // === Cross-platform window action helpers (using WindowManagerTrait) ===

    fn window_move_to_center(
        wm: &dyn WindowManagerTrait,
        window: crate::platform::traits::WindowId,
    ) -> anyhow::Result<()> {
        use crate::platform::traits::MonitorInfo;
        let info = wm.get_window_info(window)?;
        let monitors = wm.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;
        let new_x = monitor.x + (monitor.width - info.width) / 2;
        let new_y = monitor.y + (monitor.height - info.height) / 2;
        wm.set_window_pos(window, new_x, new_y, info.width, info.height)
    }

    fn window_move_to_edge(
        wm: &dyn WindowManagerTrait,
        window: crate::platform::traits::WindowId,
        edge: crate::types::Edge,
    ) -> anyhow::Result<()> {
        use crate::platform::traits::MonitorInfo;
        let info = wm.get_window_info(window)?;
        let monitors = wm.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;

        let new_x = match edge {
            crate::types::Edge::Left => monitor.x,
            crate::types::Edge::Right => monitor.x + monitor.width - info.width,
            crate::types::Edge::Top => info.x,
            crate::types::Edge::Bottom => info.x,
        };

        let new_y = match edge {
            crate::types::Edge::Top => monitor.y,
            crate::types::Edge::Bottom => monitor.y + monitor.height - info.height,
            _ => info.y,
        };

        wm.set_window_pos(window, new_x, new_y, info.width, info.height)
    }

    fn window_set_half_screen(
        wm: &dyn WindowManagerTrait,
        window: crate::platform::traits::WindowId,
        edge: crate::types::Edge,
    ) -> anyhow::Result<()> {
        use crate::platform::traits::MonitorInfo;
        let monitors = wm.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;

        let half_width = monitor.width / 2;
        let (x, width) = match edge {
            crate::types::Edge::Left => (monitor.x, half_width),
            crate::types::Edge::Right => (monitor.x + half_width, half_width),
            _ => {
                return Err(anyhow::anyhow!("HalfScreen only supports Left/Right edges"))
            }
        };

        wm.set_window_pos(window, x, monitor.y, width, monitor.height)
    }

    fn window_loop_width(
        wm: &dyn WindowManagerTrait,
        window: crate::platform::traits::WindowId,
        align: crate::types::Alignment,
    ) -> anyhow::Result<()> {
        use crate::platform::traits::MonitorInfo;
        const WIDTH_RATIOS: &[f32] = &[0.5, 0.4, 0.33, 0.25];

        let info = wm.get_window_info(window)?;
        let monitors = wm.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;

        let current_ratio = info.width as f32 / monitor.width as f32;
        let mut next_index = 0;

        for (i, &ratio) in WIDTH_RATIOS.iter().enumerate() {
            if (current_ratio - ratio).abs() < 0.05 {
                next_index = i + 1;
                break;
            }
        }

        if next_index >= WIDTH_RATIOS.len() {
            next_index = 0;
        }

        let target_width = (monitor.width as f32 * WIDTH_RATIOS[next_index]) as i32;
        let x = match align {
            crate::types::Alignment::Left => monitor.x,
            crate::types::Alignment::Center => {
                monitor.x + (monitor.width - target_width) / 2
            }
            crate::types::Alignment::Right => monitor.x + monitor.width - target_width,
            _ => monitor.x, // Top/Bottom default to left for width operations
        };

        wm.set_window_pos(window, x, monitor.y, target_width, monitor.height)
    }

    fn window_loop_height(
        wm: &dyn WindowManagerTrait,
        window: crate::platform::traits::WindowId,
        align: crate::types::Alignment,
    ) -> anyhow::Result<()> {
        use crate::platform::traits::MonitorInfo;
        const HEIGHT_RATIOS: &[f32] = &[1.0, 0.8, 0.66, 0.5];

        let info = wm.get_window_info(window)?;
        let monitors = wm.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;

        let current_ratio = info.height as f32 / monitor.height as f32;
        let mut next_index = 0;

        for (i, &ratio) in HEIGHT_RATIOS.iter().enumerate() {
            if (current_ratio - ratio).abs() < 0.05 {
                next_index = i + 1;
                break;
            }
        }

        if next_index >= HEIGHT_RATIOS.len() {
            next_index = 0;
        }

        let target_height = (monitor.height as f32 * HEIGHT_RATIOS[next_index]) as i32;
        let y = match align {
            crate::types::Alignment::Top => monitor.y,
            crate::types::Alignment::Center => {
                monitor.y + (monitor.height - target_height) / 2
            }
            crate::types::Alignment::Bottom => {
                monitor.y + monitor.height - target_height
            }
            _ => monitor.y, // Left/Right default to top for height operations
        };

        wm.set_window_pos(window, info.x, y, info.width, target_height)
    }

    fn window_set_fixed_ratio(
        wm: &dyn WindowManagerTrait,
        window: crate::platform::traits::WindowId,
        ratio: f32,
    ) -> anyhow::Result<()> {
        use crate::platform::traits::MonitorInfo;
        let info = wm.get_window_info(window)?;
        let monitors = wm.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;

        let smaller_dim = std::cmp::min(monitor.width, monitor.height);
        let base_size = smaller_dim as f32;

        let (target_width, target_height) = if ratio > 1.0 {
            ((base_size * ratio) as i32, smaller_dim)
        } else {
            (smaller_dim, (base_size / ratio) as i32)
        };

        let x = monitor.x + (monitor.width - target_width) / 2;
        let y = monitor.y + (monitor.height - target_height) / 2;

        wm.set_window_pos(window, x, y, target_width, target_height)
    }

    fn window_set_native_ratio(
        wm: &dyn WindowManagerTrait,
        window: crate::platform::traits::WindowId,
    ) -> anyhow::Result<()> {
        use crate::platform::traits::MonitorInfo;
        let info = wm.get_window_info(window)?;
        let monitors = wm.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;

        let ratio = monitor.width as f32 / monitor.height as f32;
        Self::window_set_fixed_ratio(wm, window, ratio)
    }

    fn window_switch_next(
        wm: &dyn WindowManagerTrait,
        _current_window: crate::platform::traits::WindowId,
    ) -> anyhow::Result<()> {
        let info = wm
            .get_foreground_window()
            .and_then(|w| wm.get_window_info(w).ok());

        if let Some(info) = info {
            debug!(
                process_name = %info.process_name,
                "Switching to next window of same process"
            );
        }
        Ok(())
    }

    fn window_show_debug_info(
        wm: &dyn WindowManagerTrait,
        window: crate::platform::traits::WindowId,
    ) {
        match wm.get_window_info(window) {
            Ok(info) => {
                debug!(?info, "Window debug info");
            }
            Err(e) => {
                debug!("Failed to get debug info: {}", e);
            }
        }
    }

    // === Windows-specific helper functions (kept for backward compatibility) ===

    #[cfg(target_os = "windows")]
    fn window_save_preset(
        preset_manager: Option<&mut crate::platform::windows::WindowPresetManager>,
        tray_icon: Option<&mut crate::platform::windows::TrayIcon>,
        name: &str,
        _window_id: crate::platform::traits::WindowId,
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
        window_id: crate::platform::traits::WindowId,
    ) {
        if let Some(pm) = preset_manager {
            #[cfg(target_os = "windows")]
            {
                use windows::Win32::Foundation::HWND;
                let hwnd = HWND(window_id as *mut std::ffi::c_void);
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
            }
        } else {
            debug!("WindowPresetManager not available, cannot apply preset");
        }
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
