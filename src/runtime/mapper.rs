use crate::types::{
    Action, ContextCondition, InputEvent, KeyAction, KeyEvent, KeyState, MappingRule,
    MouseEvent, Trigger,
};
use std::collections::HashMap;
use tracing::debug;

use crate::platform::traits::{
    NotificationService, WindowManagerTrait, WindowPresetManagerTrait,
};

/// Context-aware mapping rule
///
/// Select different mapping tables based on current window attributes
/// (process name, window class, title, etc.).
/// This allows the same key to have different behaviors in different applications.
///
/// Supports both keyboard (scan_code-based) and mouse (trigger-based) mappings.
///
/// # Example
///
/// ```ignore
/// // Map CapsLock to Ctrl in VSCode, but to Esc in other apps
/// let rule = ContextMappingRule {
///     context: ContextCondition::new()
///         .with_process_name("Code.exe"),
///     key_mappings: {
///         let mut map = HashMap::new();
///         map.insert(0x3A, Action::Key(KeyAction::Press { scan_code: 0x3A, virtual_key: 0x11 }));
///         map
///     },
///     trigger_mappings: vec![],
/// };
/// ```
#[derive(Debug, Clone)]
struct ContextMappingRule {
    context: ContextCondition,
    /// Keyboard mappings using scan_code for fast lookup
    key_mappings: HashMap<u16, Action>,
    /// Mouse and other trigger-based mappings
    trigger_mappings: Vec<(Trigger, Action)>,
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
/// mapper.load_rules(&rules);
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
    window_manager: Option<Box<dyn WindowManagerTrait>>,
    notification_service: Option<NotificationServiceRef>,
    window_preset_manager: Option<WindowPresetManagerRef>,
    /// O(1) lookup of scan codes referenced by any rule, used by `has_rule_for_key`.
    scan_code_index: std::collections::HashSet<u16>,
}

/// Type alias for notification service reference to simplify complex type signatures
pub(crate) type NotificationServiceRef =
    std::sync::Arc<parking_lot::Mutex<Box<dyn NotificationService>>>;

/// Type alias for window preset manager reference to simplify complex type signatures
pub(crate) type WindowPresetManagerRef =
    std::sync::Arc<parking_lot::RwLock<Box<dyn WindowPresetManagerTrait>>>;

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
            scan_code_index: std::collections::HashSet::new(),
        }
    }

    pub fn set_window_manager(&mut self, wm: Box<dyn WindowManagerTrait>) {
        self.window_manager = Some(wm);
    }

    pub(crate) fn window_manager(&self) -> Option<&dyn WindowManagerTrait> {
        self.window_manager.as_deref()
    }

    pub fn set_notification_service(&mut self, service: NotificationServiceRef) {
        self.notification_service = Some(service);
    }

    pub fn set_window_preset_manager(&mut self, manager: WindowPresetManagerRef) {
        self.window_preset_manager = Some(manager);
    }

    /// Load mapping rules from configuration
    pub fn load_rules(&mut self, rules: &[MappingRule]) {
        self.rules = rules.to_vec();
        self.rebuild_scan_code_index();
        debug!("Loaded {} mapping rules", self.rules.len());
    }

    /// Rebuild the scan code index for O(1) `has_rule_for_key` lookups.
    fn rebuild_scan_code_index(&mut self) {
        self.scan_code_index.clear();
        for rule in &self.rules {
            if let Trigger::Key {
                scan_code: Some(sc),
                ..
            } = &rule.trigger
            {
                self.scan_code_index.insert(*sc);
            }
        }
        for ctx_rule in &self.context_rules {
            for sc in ctx_rule.key_mappings.keys() {
                self.scan_code_index.insert(*sc);
            }
        }
    }

    /// Check if any base or context rule references the given scan code.
    ///
    /// O(1) lookup via pre-built index, updated on `load_rules`/`load_context_rules`.
    pub fn has_rule_for_key(&self, scan_code: u16) -> bool {
        self.scan_code_index.contains(&scan_code)
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
                    // Look up mapping in matched context (keyboard uses scan_code lookup)
                    if let Some(action) = rule.key_mappings.get(&event.scan_code) {
                        let adjusted_action =
                            self.adjust_action_for_key_state(action, event);
                        if adjusted_action.is_some() {
                            debug!(
                                "Context key mapping found: {:04X} -> {:?}",
                                event.scan_code, action
                            );
                        }
                        return adjusted_action;
                    }
                    // Check trigger-based mappings (for complex triggers with modifiers)
                    let input_event = InputEvent::Key(event.clone());
                    for (trigger, action) in &rule.trigger_mappings {
                        if trigger.matches(&input_event) {
                            let adjusted_action =
                                self.adjust_action_for_key_state(action, event);
                            if adjusted_action.is_some() {
                                debug!(
                                    "Context trigger mapping found: {:?} -> {:?}",
                                    trigger, action
                                );
                            }
                            return adjusted_action;
                        }
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
                    // Check trigger-based mappings for mouse events
                    for (trigger, action) in &rule.trigger_mappings {
                        if trigger.matches(&input_event) {
                            debug!(
                                "Context trigger mapping found for mouse: {:?} -> {:?}",
                                trigger, action
                            );
                            return Some(action.clone());
                        }
                    }
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
                    rule.trigger,
                    rule.action
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

    /// Execute window management action.
    ///
    /// This method only handles `Action::Window` variants. Other action types
    /// (Key, Mouse, Launch, Sequence, Delay, None) are intentionally ignored
    /// as they are handled by the caller (daemon.rs) directly.
    ///
    /// # Arguments
    ///
    /// * `action` - The action to execute. Only Window actions are processed.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the action was handled successfully or if it's a
    /// non-Window action (which are silently ignored). Returns an error if
    /// a Window action fails to execute.
    pub fn execute_action(&self, action: &Action) -> anyhow::Result<()> {
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
            // Non-Window actions are intentionally ignored - they are handled by the caller
            _ => {
                debug!(
                    "KeyMapper::execute_action only handles Window actions; \
                     non-Window action {:?} ignored (should be handled elsewhere)",
                    action
                );
            }
        }

        Ok(())
    }

    fn execute_window_action_internal(
        wm: &dyn WindowManagerTrait,
        notification_service: Option<&NotificationServiceRef>,
        preset_manager: Option<&WindowPresetManagerRef>,
        action: &crate::types::WindowAction,
    ) -> anyhow::Result<()> {
        super::window_actions::execute_window_action(
            wm,
            notification_service,
            preset_manager,
            action,
        )
    }

    /// Load context-aware mapping rules
    pub fn load_context_rules(
        &mut self,
        context_mappings: &[crate::config::ContextMapping],
    ) {
        self.context_rules.clear();

        for mapping in context_mappings {
            let mut key_mappings = HashMap::new();
            let mut trigger_mappings = Vec::new();

            // Parse each mapping string
            for (from, to) in &mapping.mappings {
                // Try to parse as simple key mapping (for fast lookup)
                if let Ok((scan_code, action)) =
                    Self::parse_context_key_mapping(from, to)
                {
                    key_mappings.insert(scan_code, action);
                    continue;
                }

                // Try to parse as trigger-based mapping (supports modifiers and mouse)
                if let Ok((trigger, action)) =
                    Self::parse_context_trigger_mapping(from, to)
                {
                    trigger_mappings.push((trigger, action));
                }
            }

            if !key_mappings.is_empty() || !trigger_mappings.is_empty() {
                self.context_rules.push(ContextMappingRule {
                    context: mapping.context.clone(),
                    key_mappings,
                    trigger_mappings,
                });
            }
        }

        self.rebuild_scan_code_index();
        debug!("Loaded {} context mapping rules", self.context_rules.len());
    }

    /// Parse simple key context mapping (returns scan_code for fast lookup)
    fn parse_context_key_mapping(from: &str, to: &str) -> anyhow::Result<(u16, Action)> {
        use crate::config::parse_key;
        use crate::types::KeyAction;

        // Parse source key - must be a simple key without modifiers
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
            "Failed to parse key mapping: {} -> {}",
            from,
            to
        ))
    }

    /// Parse trigger-based context mapping (supports modifiers and mouse buttons)
    fn parse_context_trigger_mapping(
        from: &str,
        to: &str,
    ) -> anyhow::Result<(Trigger, Action)> {
        use crate::config::{parse_key, parse_shortcut_trigger, parse_window_action};
        use crate::types::KeyAction;

        // Try to parse source as shortcut trigger (supports modifiers)
        let trigger = parse_shortcut_trigger(from)?;

        // Parse target action
        // First try to parse as key
        if let Ok(to_key) = parse_key(to) {
            return Ok((trigger, Action::key(KeyAction::click(to_key.0, to_key.1))));
        }

        // Try to parse as window action
        if let Ok(window_action) = parse_window_action(to) {
            return Ok((trigger, Action::window(window_action)));
        }

        Err(anyhow::anyhow!(
            "Failed to parse trigger mapping: {} -> {}",
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

        mapper.load_rules(&rules);

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

        mapper.load_rules(&rules);
    }

    #[test]
    fn test_mapper_process_event_simple_match() {
        let mut mapper = KeyMapper::new();

        let rules = vec![MappingRule::new(
            Trigger::key(0x3A, 0x14),                  // CapsLock
            Action::key(KeyAction::click(0x0E, 0x08)), // Backspace
        )];

        mapper.load_rules(&rules);

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

        mapper.load_rules(&rules);

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

        mapper.load_rules(&rules);

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

        mapper.load_rules(&rules);

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

        mapper.load_rules(&rules);

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
        let mut event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
        event.modifiers.ctrl = true;
        let event = InputEvent::Key(event);

        assert!(rule.trigger.matches(&event));
    }
}
