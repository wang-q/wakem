use crate::platform::traits::{NotificationService, WindowManager, WindowPresetManager};
use crate::types::{
    Action, ContextCondition, InputEvent, KeyAction, KeyEvent, KeyState, MappingRule,
    MonitorDirection, WindowAction,
};
use std::collections::HashMap;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ContextMappingRule {
    pub context: ContextCondition,
    pub mappings: HashMap<u16, Action>,
}

pub struct KeyMapper {
    rules: Vec<MappingRule>,
    context_rules: Vec<ContextMappingRule>,
    enabled: bool,
    pub(crate) window_manager: Option<Box<dyn WindowManager>>,
    notification_service: Option<Box<dyn NotificationService>>,
    window_preset_manager: Option<Box<dyn WindowPresetManager>>,
}

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

    pub fn with_window_manager(
        window_manager: Box<dyn WindowManager>,
        notification_service: Option<Box<dyn NotificationService>>,
        window_preset_manager: Option<Box<dyn WindowPresetManager>>,
    ) -> Self {
        Self {
            rules: Vec::new(),
            context_rules: Vec::new(),
            enabled: true,
            window_manager: Some(window_manager),
            notification_service,
            window_preset_manager,
        }
    }

    pub fn load_rules(&mut self, rules: Vec<MappingRule>) {
        self.rules = rules;
        debug!("Loaded {} mapping rules", self.rules.len());
    }

    pub fn process_event_with_context(
        &self,
        event: &InputEvent,
        context: Option<&crate::platform::types::WindowContext>,
    ) -> Option<Action> {
        if !self.enabled {
            return None;
        }

        match event {
            InputEvent::Key(key_event) => {
                self.process_key_event_with_context(key_event, context)
            }
            InputEvent::Mouse(_) => None,
        }
    }

    fn process_key_event_with_context(
        &self,
        event: &KeyEvent,
        context: Option<&crate::platform::types::WindowContext>,
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

        if let Some(ctx) = context {
            debug!(
                process_name = %ctx.process_name,
                window_class = %ctx.window_class,
                "Checking context rules"
            );
            for rule in &self.context_rules {
                if rule.context.matches(
                    &ctx.process_name,
                    &ctx.window_class,
                    &ctx.window_title,
                    ctx.executable_path.as_deref(),
                ) {
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
            ) => match event.state {
                KeyState::Pressed => Some(Action::Key(KeyAction::Press {
                    scan_code: *scan_code,
                    virtual_key: *virtual_key,
                })),
                KeyState::Released => Some(Action::Key(KeyAction::Release {
                    scan_code: *scan_code,
                    virtual_key: *virtual_key,
                })),
            },
            (Action::Sequence(actions), _) if actions.len() > 1 => {
                let noop_position =
                    actions.iter().position(|a| matches!(a, Action::None));

                if let Some(pos) = noop_position {
                    match event.state {
                        KeyState::Pressed => {
                            let press_actions: Vec<_> =
                                actions.iter().take(pos).cloned().collect();
                            if press_actions.is_empty() {
                                None
                            } else {
                                Some(Action::Sequence(press_actions))
                            }
                        }
                        KeyState::Released => {
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
                    execute_window_action_impl(
                        wm.as_ref(),
                        window_action,
                        &self.notification_service,
                        &mut self.window_preset_manager,
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

    pub fn load_context_rules(
        &mut self,
        context_mappings: &[crate::config::ContextMapping],
    ) {
        self.context_rules.clear();

        for mapping in context_mappings {
            let mut rule_mappings = HashMap::new();

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

    fn parse_context_mapping(from: &str, to: &str) -> anyhow::Result<(u16, Action)> {
        use crate::config::parse_key;
        use crate::types::KeyAction;

        let from_key = parse_key(from)?;

        if let Ok(to_key) = parse_key(to) {
            return Ok((
                from_key.scan_code,
                Action::key(KeyAction::click(to_key.scan_code, to_key.virtual_key)),
            ));
        }

        if let Ok(window_action) = crate::config::parse_window_action(to) {
            return Ok((from_key.scan_code, Action::window(window_action)));
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

fn execute_window_action_impl(
    wm: &dyn WindowManager,
    action: &WindowAction,
    notification_service: &Option<Box<dyn NotificationService>>,
    window_preset_manager: &mut Option<Box<dyn WindowPresetManager>>,
) -> anyhow::Result<()> {
    use crate::platform::common::window_ops;

    let window = wm
        .get_foreground_window()
        .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;

    match action {
        WindowAction::Center => {
            let info = wm.get_window_info(window)?;
            let monitors = wm.get_monitors();
            if let Some((x, y)) = window_ops::calc_centered_pos(&info, &monitors) {
                wm.set_window_pos(window, x, y, info.width, info.height)?;
            }
        }
        WindowAction::MoveToEdge(edge) => {
            let info = wm.get_window_info(window)?;
            let monitors = wm.get_monitors();
            if let Some((x, y)) = window_ops::calc_edge_pos(&info, &monitors, *edge) {
                wm.set_window_pos(window, x, y, info.width, info.height)?;
            }
        }
        WindowAction::HalfScreen(edge) => {
            let info = wm.get_window_info(window)?;
            let monitors = wm.get_monitors();
            if let Some((x, y, w, h)) =
                window_ops::calc_half_screen(&info, &monitors, *edge)
            {
                wm.set_window_pos(window, x, y, w, h)?;
            }
        }
        WindowAction::LoopWidth(align) => {
            let info = wm.get_window_info(window)?;
            let monitors = wm.get_monitors();
            if let Some((x, y, w, h)) =
                window_ops::calc_looped_width(&info, &monitors, *align)
            {
                wm.set_window_pos(window, x, y, w, h)?;
            }
        }
        WindowAction::LoopHeight(align) => {
            let info = wm.get_window_info(window)?;
            let monitors = wm.get_monitors();
            if let Some((x, y, w, h)) =
                window_ops::calc_looped_height(&info, &monitors, *align)
            {
                wm.set_window_pos(window, x, y, w, h)?;
            }
        }
        WindowAction::FixedRatio { ratio, scale_index } => {
            let info = wm.get_window_info(window)?;
            let monitors = wm.get_monitors();
            if let Some((x, y, w, h)) = window_ops::calc_fixed_ratio(
                &info,
                &monitors,
                *ratio,
                Some(*scale_index),
            ) {
                wm.set_window_pos(window, x, y, w, h)?;
            }
        }
        WindowAction::NativeRatio { scale_index } => {
            let info = wm.get_window_info(window)?;
            let monitors = wm.get_monitors();
            if let Some((x, y, w, h)) =
                window_ops::calc_native_ratio(&info, &monitors, Some(*scale_index))
            {
                wm.set_window_pos(window, x, y, w, h)?;
            }
        }
        WindowAction::SwitchToNextWindow => {
            wm.switch_to_next_window_of_same_process()?;
        }
        WindowAction::MoveToMonitor(direction) => {
            execute_move_to_monitor(wm, window, direction)?;
        }
        WindowAction::Move { x, y } => {
            let info = wm.get_window_info(window)?;
            wm.set_window_pos(window, *x, *y, info.width, info.height)?;
        }
        WindowAction::Resize { width, height } => {
            let info = wm.get_window_info(window)?;
            wm.set_window_pos(window, info.x, info.y, *width, *height)?;
        }
        WindowAction::Minimize => wm.minimize_window(window)?,
        WindowAction::Maximize => wm.maximize_window(window)?,
        WindowAction::Restore => wm.restore_window(window)?,
        WindowAction::Close => wm.close_window(window)?,
        WindowAction::ToggleTopmost => {
            let current = wm.is_topmost(window);
            wm.set_topmost(window, !current)?;
        }
        WindowAction::ShowDebugInfo => {
            show_debug_info(wm, window, notification_service);
        }
        WindowAction::ShowNotification { title, message } => {
            show_notification(title, message, notification_service);
        }
        WindowAction::SavePreset { name } => {
            save_preset(name, notification_service, window_preset_manager);
        }
        WindowAction::LoadPreset { name } => {
            load_preset(window, name, window_preset_manager);
        }
        WindowAction::ApplyPreset => {
            apply_preset(window, window_preset_manager);
        }
        WindowAction::None => {}
    }

    Ok(())
}

fn execute_move_to_monitor(
    wm: &dyn WindowManager,
    window: crate::platform::types::WindowId,
    direction: &MonitorDirection,
) -> anyhow::Result<()> {
    let monitors = wm.get_monitors();
    if monitors.len() <= 1 {
        debug!("Only one monitor, skipping move");
        return Ok(());
    }

    let info = wm.get_window_info(window)?;
    let current_monitor_idx = monitors
        .iter()
        .position(|m| {
            info.x >= m.x
                && info.x < m.x + m.width
                && info.y >= m.y
                && info.y < m.y + m.height
        })
        .unwrap_or(0);

    let target_index = match direction {
        MonitorDirection::Next => (current_monitor_idx + 1) % monitors.len(),
        MonitorDirection::Prev => {
            if current_monitor_idx == 0 {
                monitors.len() - 1
            } else {
                current_monitor_idx - 1
            }
        }
        MonitorDirection::Index(idx) => {
            if *idx >= 0 && (*idx as usize) < monitors.len() {
                *idx as usize
            } else {
                current_monitor_idx
            }
        }
    };

    if target_index == current_monitor_idx {
        debug!("Already on target monitor, skipping move");
        return Ok(());
    }

    let target = &monitors[target_index];
    let current = &monitors[current_monitor_idx];
    let rel_x = (info.x - current.x) as f32 / current.width as f32;
    let rel_y = (info.y - current.y) as f32 / current.height as f32;
    let new_x = target.x + (rel_x * target.width as f32) as i32;
    let new_y = target.y + (rel_y * target.height as f32) as i32;
    wm.set_window_pos(window, new_x, new_y, info.width, info.height)
}

fn show_debug_info(
    wm: &dyn WindowManager,
    window: crate::platform::types::WindowId,
    notification_service: &Option<Box<dyn NotificationService>>,
) {
    match wm.get_window_info(window) {
        Ok(info) => {
            let debug_info = format!(
                "Window Debug Info:\n\
                 Position: ({}, {})\n\
                 Size: {}x{}\n\
                 Process: {}",
                info.x, info.y, info.width, info.height, info.process_name
            );
            debug!("{}", debug_info);
            show_notification("wakem - Debug Info", &debug_info, notification_service);
        }
        Err(e) => {
            debug!("Failed to get debug info: {}", e);
        }
    }
}

fn show_notification(
    title: &str,
    message: &str,
    notification_service: &Option<Box<dyn NotificationService>>,
) {
    if let Some(ref ns) = notification_service {
        if let Err(e) = ns.show(title, message) {
            debug!("Failed to show notification: {}", e);
        }
    } else {
        debug!("NotificationService not available, cannot show notification");
    }
}

fn save_preset(
    name: &str,
    notification_service: &Option<Box<dyn NotificationService>>,
    window_preset_manager: &mut Option<Box<dyn WindowPresetManager>>,
) {
    if let Some(ref mut pm) = window_preset_manager {
        match pm.get_foreground_window_info() {
            Some(Ok(_info)) => {
                if let Err(e) = pm.save_preset(name.to_string()) {
                    debug!("Failed to save preset '{}': {}", name, e);
                } else {
                    debug!("Saved preset '{}' for current window", name);
                    show_notification(
                        "wakem",
                        &format!("Preset '{}' saved", name),
                        notification_service,
                    );
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

fn load_preset(
    _window: crate::platform::types::WindowId,
    name: &str,
    window_preset_manager: &Option<Box<dyn WindowPresetManager>>,
) {
    if let Some(ref pm) = window_preset_manager {
        if let Err(e) = pm.load_preset(name) {
            debug!("Failed to load preset '{}': {}", name, e);
        } else {
            debug!("Loaded preset '{}' for current window", name);
        }
    } else {
        debug!("WindowPresetManager not available, cannot load preset");
    }
}

fn apply_preset(
    window: crate::platform::types::WindowId,
    window_preset_manager: &Option<Box<dyn WindowPresetManager>>,
) {
    if let Some(ref pm) = window_preset_manager {
        match pm.apply_preset_for_window_by_id(window) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{InputEvent, KeyEvent, KeyState, ModifierState, Trigger};

    #[test]
    fn test_key_mapper_basic() {
        let mapper = KeyMapper::new();
        assert!(mapper.enabled);
    }

    #[test]
    fn test_key_mapper_disabled() {
        let mut mapper = KeyMapper::new();
        mapper.enabled = false;

        let event = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
        let result = mapper.process_event_with_context(&InputEvent::Key(event), None);

        assert!(result.is_none());
    }

    #[test]
    fn test_key_mapper_load_rules() {
        let mut mapper = KeyMapper::new();

        let rules = vec![MappingRule::new(
            Trigger::key(0x3A, 0x14),
            Action::key(KeyAction::click(0x0E, 0x08)),
        )];

        mapper.load_rules(rules);
        assert_eq!(mapper.rules.len(), 1);
    }

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
                Trigger::key(0x3A, 0x14),
                Action::key(KeyAction::click(0x0E, 0x08)),
            ),
            MappingRule::new(
                Trigger::key(0x01, 0x1B),
                Action::key(KeyAction::click(0x4B, 0x25)),
            ),
        ];

        mapper.load_rules(rules);
    }

    #[test]
    fn test_mapper_process_event_simple_match() {
        let mut mapper = KeyMapper::new();

        let rules = vec![MappingRule::new(
            Trigger::key(0x3A, 0x14),
            Action::key(KeyAction::click(0x0E, 0x08)),
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
            Trigger::key(0x3A, 0x14),
            Action::key(KeyAction::click(0x0E, 0x08)),
        )];

        mapper.load_rules(rules);

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

        let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
        let result = mapper.process_event_with_context(&event, None);
        assert!(result.is_some());
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
            Action::key(KeyAction::click(0x0E, 0x08)),
        )];

        mapper.load_rules(rules);

        let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
        let result = mapper.process_event_with_context(&event, None);

        assert!(result.is_some());
        if let Some(Action::Key(KeyAction::Press { .. })) = result {
        } else {
            panic!("Press event should convert to Press action");
        }
    }

    #[test]
    fn test_mapper_adjust_action_released() {
        let mut mapper = KeyMapper::new();

        let rules = vec![MappingRule::new(
            Trigger::key(0x3A, 0x14),
            Action::key(KeyAction::click(0x0E, 0x08)),
        )];

        mapper.load_rules(rules);

        let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Released));
        let result = mapper.process_event_with_context(&event, None);

        assert!(result.is_some());
        if let Some(Action::Key(KeyAction::Release { .. })) = result {
        } else {
            panic!("Release event should convert to Release action");
        }
    }

    #[test]
    fn test_mapping_rule_matching() {
        let rule = MappingRule::new(
            Trigger::key(0x1E, 0x41),
            Action::key(KeyAction::click(0x1F, 0x42)),
        );

        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        assert!(rule.trigger.matches(&event));
    }

    #[test]
    fn test_mapping_rule_with_modifiers() {
        let mut modifiers = ModifierState::new();
        modifiers.ctrl = true;
        let trigger = Trigger::key_with_modifiers(0x1E, 0x41, modifiers);

        let rule = MappingRule::new(trigger, Action::key(KeyAction::click(0x1F, 0x42)));

        let mut event_modifiers = ModifierState::new();
        event_modifiers.ctrl = true;
        let event = InputEvent::Key(
            KeyEvent::new(0x1E, 0x41, KeyState::Pressed).with_modifiers(event_modifiers),
        );

        assert!(rule.trigger.matches(&event));
    }
}
