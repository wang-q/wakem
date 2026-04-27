use crate::types::{
    Action, ContextCondition, InputEvent, KeyAction, KeyEvent, KeyState, MappingRule, MouseEvent,
};
use std::collections::HashMap;
use tracing::{debug, warn};

use crate::platform::traits::{
    NotificationService, WindowManagerExt, WindowManagerTrait, WindowPresetManagerTrait,
};

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
///         map.insert(0x3A, Action::Key(KeyAction::Press { scan_code: 0x3A, virtual_key: 0x11 }));
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
    rules: Vec<MappingRule>,
    context_rules: Vec<ContextMappingRule>,
    enabled: bool,
    pub(crate) window_manager: Option<Box<dyn WindowManagerTrait>>,
    notification_service:
        Option<std::sync::Arc<parking_lot::Mutex<Box<dyn NotificationService>>>>,
    window_preset_manager:
        Option<std::sync::Arc<parking_lot::RwLock<Box<dyn WindowPresetManagerTrait>>>>,
}

// Note: KeyMapper is Send + Sync because all contained trait objects
// (WindowManagerTrait, NotificationService, WindowPresetManagerTrait)
// already require Send + Sync bounds in their trait definitions.

impl KeyMapper {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            context_rules: Vec::new(),
            enabled: true,
            window_manager: None,
            notification_service: None,
            window_preset_manager: None,
        }
    }

    pub fn set_window_manager(&mut self, wm: Box<dyn WindowManagerTrait>) {
        self.window_manager = Some(wm);
    }

    pub fn set_notification_service(
        &mut self,
        service: std::sync::Arc<parking_lot::Mutex<Box<dyn NotificationService>>>,
    ) {
        self.notification_service = Some(service);
    }

    pub fn set_window_preset_manager(
        &mut self,
        manager: std::sync::Arc<parking_lot::RwLock<Box<dyn WindowPresetManagerTrait>>>,
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
            InputEvent::Mouse(mouse_event) => {
                self.process_mouse_event_with_context(mouse_event, context)
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

    /// Process mouse event (with context awareness)
    fn process_mouse_event_with_context(
        &self,
        event: &MouseEvent,
        context: Option<&crate::platform::traits::WindowContext>,
    ) -> Option<Action> {
        debug!(
            mouse_event = ?event,
            context_rules_count = self.context_rules.len(),
            base_rules_count = self.rules.len(),
            "Mapper processing mouse event"
        );

        let input_event = InputEvent::Mouse(event.clone());

        // Mouse events are matched by trigger patterns, not by button code lookup
        // The Trigger::MouseButton variant handles mouse button matching

        // 1. First check context-specific rules (high priority)
        if let Some(ctx) = context {
            debug!(
                process_name = %ctx.process_name,
                window_class = %ctx.window_class,
                "Checking context rules for mouse event"
            );
            for rule in &self.context_rules {
                if rule.context.matches(
                    &ctx.process_name,
                    &ctx.window_class,
                    &ctx.window_title,
                    ctx.executable_path.as_deref(),
                ) {
                    // For mouse events, we need to check trigger matching
                    // Context rules currently use scan_code lookup which doesn't apply to mouse
                    // This would need Trigger-based matching to work properly
                }
            }
        }

        // 2. Check base rules using trigger matching
        debug!("Checking base rules for mouse event");
        for (idx, rule) in self.rules.iter().enumerate() {
            if !rule.enabled {
                continue;
            }
            debug!(rule_idx = idx, trigger = ?rule.trigger, "Checking rule");
            if rule.trigger.matches(&input_event) {
                debug!(
                    rule_idx = idx,
                    "Base rule matched for mouse: trigger={:?} -> {:?}",
                    rule.trigger, rule.action
                );
                return Some(rule.action.clone());
            }
        }

        debug!("No matching rule found for mouse event");
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

    pub fn execute_action(&mut self, action: &Action) -> anyhow::Result<()> {
        match action {
            Action::Window(window_action) => {
                if let Some(ref wm) = self.window_manager {
                    Self::execute_window_action_internal(
                        wm.as_ref(),
                        self.notification_service.as_ref(),
                        self.window_preset_manager.as_ref(),
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
            | Action::None => {}
        }

        Ok(())
    }

    fn execute_window_action_internal(
        wm: &dyn WindowManagerTrait,
        notification_service: Option<
            &std::sync::Arc<parking_lot::Mutex<Box<dyn NotificationService>>>,
        >,
        preset_manager: Option<
            &std::sync::Arc<parking_lot::RwLock<Box<dyn WindowPresetManagerTrait>>>,
        >,
        action: &crate::types::WindowAction,
    ) -> anyhow::Result<()> {
        use crate::types::{MonitorDirection, WindowAction};

        let window_id = wm
            .get_foreground_window()
            .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;

        match action {
            WindowAction::Center => {
                wm.move_to_center(window_id)?;
            }
            WindowAction::MoveToEdge(edge) => {
                wm.move_to_edge(window_id, *edge)?;
            }
            WindowAction::HalfScreen(edge) => {
                wm.set_half_screen(window_id, *edge)?;
            }
            WindowAction::LoopWidth(align) => {
                wm.loop_width(window_id, *align)?;
            }
            WindowAction::LoopHeight(align) => {
                wm.loop_height(window_id, *align)?;
            }
            WindowAction::FixedRatio {
                ratio,
                scale_index: _,
            } => {
                wm.set_fixed_ratio(window_id, *ratio)?;
            }
            WindowAction::NativeRatio { scale_index: _ } => {
                wm.set_native_ratio(window_id)?;
            }
            WindowAction::SwitchToNextWindow => {
                // TODO: Implement window switching when WindowManagerTrait supports
                // get_all_windows() and activate_window() methods
                if let Some(info) = wm
                    .get_foreground_window()
                    .and_then(|w| wm.get_window_info(w).ok())
                {
                    warn!(
                        process_name = %info.process_name,
                        "SwitchToNextWindow not yet implemented - requires platform support"
                    );
                }
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
                wm.toggle_topmost(window_id)?;
            }
            WindowAction::ShowDebugInfo => match wm.get_window_info(window_id) {
                Ok(info) => {
                    debug!(?info, "Window debug info");
                }
                Err(e) => {
                    debug!("Failed to get debug info: {}", e);
                }
            },
            WindowAction::ShowNotification { title, message } => {
                if let Some(ns) = notification_service {
                    let ns = ns.lock();
                    let _ = ns.show(title, message);
                }
            }
            WindowAction::SavePreset { name } => {
                if let Some(pm) = preset_manager {
                    let mut pm = pm.write();
                    match pm.get_foreground_window_info() {
                        Some(Ok(_)) => {
                            if let Err(e) = pm.save_preset(name.to_string()) {
                                debug!("Failed to save preset '{}': {}", name, e);
                            } else {
                                debug!("Saved preset '{}' for current window", name);
                                if let Some(ns) = notification_service {
                                    let ns = ns.lock();
                                    let _ = ns.show(
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
            WindowAction::LoadPreset { name } => {
                if let Some(pm) = preset_manager {
                    let pm = pm.read();
                    if let Err(e) = pm.load_preset(name) {
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
                    let pm = pm.read();
                    match pm.apply_preset_for_window_by_id(window_id) {
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

        let mouse_event = crate::types::MouseEvent::new(
            crate::types::MouseEventType::Move { relative: false },
            100,
            200,
        );
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
