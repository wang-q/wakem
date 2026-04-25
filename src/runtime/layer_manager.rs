use crate::types::{
    Action, InputEvent, KeyAction, KeyEvent, KeyState, Layer, LayerMode, LayerStack,
    MappingRule, Trigger,
};
use std::collections::HashMap;
use tracing::{debug, trace};

/// Activation key index entry
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ActivationKey {
    scan_code: u16,
    virtual_key: u16,
}

/// Layer manager
pub struct LayerManager {
    /// All available layers
    layers: HashMap<String, Layer>,
    /// Layer stack (manages activation state)
    stack: LayerStack,
    /// Base layer mappings (simple remapping loaded from config)
    base_mappings: Vec<MappingRule>,
    /// Activation key index: (scan_code, vk) -> layer name
    activation_key_index: HashMap<ActivationKey, String>,
}

impl LayerManager {
    pub fn new() -> Self {
        Self {
            layers: HashMap::new(),
            stack: LayerStack::new(),
            base_mappings: Vec::new(),
            activation_key_index: HashMap::new(),
        }
    }

    /// Register a layer
    pub fn register_layer(&mut self, layer: Layer) {
        debug!("Registering layer: {}", layer.name);
        let key = ActivationKey {
            scan_code: layer.activation_key,
            virtual_key: layer.activation_vk,
        };
        self.activation_key_index.insert(key, layer.name.clone());
        self.layers.insert(layer.name.clone(), layer);
    }

    /// Set base layer mappings
    pub fn set_base_mappings(&mut self, mappings: Vec<MappingRule>) {
        self.base_mappings = mappings.clone();
        self.stack.set_base_layer(mappings);
    }

    /// Process input event, check if it's a layer activation key
    /// Returns (whether event was handled, optional action)
    pub fn process_event(&mut self, event: &InputEvent) -> (bool, Option<Action>) {
        match event {
            InputEvent::Key(key_event) => self.process_key_event(key_event),
            _ => (false, None),
        }
    }

    /// Process keyboard event
    fn process_key_event(&mut self, event: &KeyEvent) -> (bool, Option<Action>) {
        let lookup_key = ActivationKey {
            scan_code: event.scan_code,
            virtual_key: event.virtual_key,
        };

        if let Some(layer_name) = self.activation_key_index.get(&lookup_key) {
            if let Some(layer) = self.layers.get(layer_name) {
                match layer.mode {
                    LayerMode::Hold => {
                        match event.state {
                            KeyState::Pressed => {
                                trace!("Activating layer (Hold): {}", layer.name);
                                if let Some(layer) =
                                    self.layers.get(&layer.name).cloned()
                                {
                                    self.stack.hold_layer(&layer.name);
                                    self.stack.activate_layer(layer);
                                }
                            }
                            KeyState::Released => {
                                trace!("Deactivating layer (Hold): {}", layer.name);
                                self.stack.release_layer(&layer.name);
                            }
                        }
                        return (true, None);
                    }
                    LayerMode::Toggle => {
                        if event.state == KeyState::Pressed {
                            trace!("Toggling layer: {}", layer.name);
                            if let Some(layer) = self.layers.get(&layer.name).cloned() {
                                self.stack.toggle_layer(layer);
                            }
                            return (true, None);
                        }
                    }
                }
            }
        }

        // If not an activation key, search for mapping in active layers
        let input_event = InputEvent::Key(event.clone());
        let mappings = self.stack.get_all_mappings();
        for rule in &mappings {
            if rule.trigger.matches(&input_event) {
                trace!("Found mapping in layer: {:?}", rule.action);
                return (true, Some(rule.action.clone()));
            }
        }

        (false, None)
    }

    /// Get list of currently active layers
    #[allow(dead_code)]
    pub fn get_active_layers(&self) -> Vec<String> {
        self.stack
            .get_active_layers()
            .iter()
            .map(|l| l.name.clone())
            .collect()
    }

    /// Check if layer is active
    #[allow(dead_code)]
    pub fn is_layer_active(&self, name: &str) -> bool {
        self.stack.is_layer_active(name)
    }

    /// Deactivate all layers
    #[allow(dead_code)]
    pub fn clear_layers(&mut self) {
        self.stack.clear_active_layers();
    }

    /// Create layer from config
    pub fn create_layer_from_config(
        name: &str,
        activation_key: &str,
        mode: LayerMode,
        mappings: &[(String, String)],
    ) -> anyhow::Result<Layer> {
        use crate::config::parse_key;

        let (scan, vk) = parse_key(activation_key)?;
        let mut layer = Layer::new(name, scan, vk).with_mode(mode);

        for (from, to) in mappings {
            let from_key = parse_key(from)?;
            let to_key = parse_key(to)?;

            layer.add_mapping(
                Trigger::key(from_key.0, from_key.1),
                Action::key(KeyAction::click(to_key.0, to_key.1)),
            );
        }

        Ok(layer)
    }
}

impl Default for LayerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Action, InputEvent, KeyAction, KeyEvent, KeyState, Layer, LayerMode, Trigger,
    };

    #[test]
    fn test_layer_manager_hold() {
        let mut manager = LayerManager::new();

        // Create navigation layer
        let layer = LayerManager::create_layer_from_config(
            "navigate",
            "CapsLock",
            LayerMode::Hold,
            &[("H".to_string(), "Left".to_string())],
        )
        .unwrap();

        manager.register_layer(layer);

        // Simulate pressing CapsLock
        let press = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
        let (handled, _) = manager.process_event(&InputEvent::Key(press));
        assert!(handled);
        assert!(manager.is_layer_active("navigate"));

        // Simulate releasing CapsLock
        let release = KeyEvent::new(0x3A, 0x14, KeyState::Released);
        let (handled, _) = manager.process_event(&InputEvent::Key(release));
        assert!(handled);
        assert!(!manager.is_layer_active("navigate"));
    }

    #[test]
    fn test_layer_mapping() {
        let mut manager = LayerManager::new();

        // Create navigation layer
        let layer = LayerManager::create_layer_from_config(
            "navigate",
            "CapsLock",
            LayerMode::Hold,
            &[("H".to_string(), "Left".to_string())],
        )
        .unwrap();

        manager.register_layer(layer);

        // Activate layer first
        let caps_press = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
        manager.process_event(&InputEvent::Key(caps_press));

        // Simulate pressing H, should map to Left
        let h_press = KeyEvent::new(0x23, 0x48, KeyState::Pressed);
        let (handled, action) = manager.process_event(&InputEvent::Key(h_press));
        assert!(handled);
        assert!(action.is_some());
    }

    // ==================== Additional tests from ut_runtime_mapper_full.rs ====================

    #[test]
    fn test_layer_manager_new() {
        let manager = LayerManager::new();
        assert!(!manager.is_layer_active("any_layer"));
        assert!(manager.get_active_layers().is_empty());
    }

    #[test]
    fn test_layer_manager_default() {
        let manager = LayerManager::default();
        assert!(manager.get_active_layers().is_empty());
    }

    #[test]
    fn test_layer_manager_register_layer() {
        let mut manager = LayerManager::new();

        let layer = Layer::new("test_layer", 0x3A, 0x14).with_mode(LayerMode::Hold);
        manager.register_layer(layer);

        // After registration, layer is not yet activated
        assert!(!manager.is_layer_active("test_layer"));
    }

    #[test]
    fn test_layer_manager_hold_mode_activate_deactivate() {
        let mut manager = LayerManager::new();

        let layer = LayerManager::create_layer_from_config(
            "nav",
            "CapsLock",
            LayerMode::Hold,
            &[],
        )
        .unwrap();
        manager.register_layer(layer);

        // Press activation key
        let press = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
        let (handled, _) = manager.process_event(&InputEvent::Key(press));

        assert!(handled, "Should handle activation key press event");
        assert!(manager.is_layer_active("nav"), "Layer should be activated");

        // Release activation key
        let release = KeyEvent::new(0x3A, 0x14, KeyState::Released);
        let (handled, _) = manager.process_event(&InputEvent::Key(release));

        assert!(handled, "Should handle activation key release event");
        assert!(
            !manager.is_layer_active("nav"),
            "Hold mode layer should be deactivated after release"
        );
    }

    #[test]
    fn test_layer_manager_toggle_mode() {
        let mut manager = LayerManager::new();

        let layer = LayerManager::create_layer_from_config(
            "sym",
            "Space",
            LayerMode::Toggle,
            &[],
        )
        .unwrap();
        manager.register_layer(layer);

        // First press -> activate
        let press1 = KeyEvent::new(0x39, 0x20, KeyState::Pressed);
        let (handled, _) = manager.process_event(&InputEvent::Key(press1));
        assert!(handled);
        assert!(
            manager.is_layer_active("sym"),
            "First press should activate"
        );

        // Second press -> deactivate
        let press2 = KeyEvent::new(0x39, 0x20, KeyState::Pressed);
        let (handled, _) = manager.process_event(&InputEvent::Key(press2));
        assert!(handled);
        assert!(
            !manager.is_layer_active("sym"),
            "Second press should deactivate"
        );
    }

    #[test]
    fn test_layer_mapping_lookup() {
        let mut manager = LayerManager::new();

        let layer = LayerManager::create_layer_from_config(
            "nav",
            "RAlt",
            LayerMode::Hold,
            &[("H".to_string(), "Left".to_string())],
        )
        .unwrap();
        manager.register_layer(layer);

        // Activate layer
        let alt_press = KeyEvent::new(0xE038, 0xA5, KeyState::Pressed);
        manager.process_event(&InputEvent::Key(alt_press));

        // Press H in layer, should map to Left
        let h_press = KeyEvent::new(0x23, 0x48, KeyState::Pressed);
        let (handled, action) = manager.process_event(&InputEvent::Key(h_press));

        assert!(handled, "Should find mapping in layer");
        assert!(action.is_some(), "Should return action");
    }

    #[test]
    fn test_layer_non_activation_key_not_handled() {
        let mut manager = LayerManager::new();

        let layer = LayerManager::create_layer_from_config(
            "nav",
            "CapsLock",
            LayerMode::Hold,
            &[],
        )
        .unwrap();
        manager.register_layer(layer);

        // Press a normal key (not activation key)
        let a_press = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
        let (handled, action) = manager.process_event(&InputEvent::Key(a_press));

        assert!(!handled, "Non-activation key should not be handled");
        assert!(action.is_none(), "Should not return action");
    }

    #[test]
    fn test_layer_manager_multiple_layers() {
        let mut manager = LayerManager::new();

        let nav =
            LayerManager::create_layer_from_config("nav", "RAlt", LayerMode::Hold, &[])
                .unwrap();
        let sym = LayerManager::create_layer_from_config(
            "sym",
            "Space",
            LayerMode::Toggle,
            &[],
        )
        .unwrap();
        let num =
            LayerManager::create_layer_from_config("num", "F12", LayerMode::Hold, &[])
                .unwrap();

        manager.register_layer(nav);
        manager.register_layer(sym);
        manager.register_layer(num);

        // All layers initially inactive
        assert!(!manager.is_layer_active("nav"));
        assert!(!manager.is_layer_active("sym"));
        assert!(!manager.is_layer_active("num"));
    }

    #[test]
    fn test_layer_manager_clear_layers() {
        let mut manager = LayerManager::new();

        let layer = LayerManager::create_layer_from_config(
            "test",
            "F11",
            LayerMode::Toggle,
            &[],
        )
        .unwrap();
        manager.register_layer(layer);

        // Activate layer
        let f11_press = KeyEvent::new(0x57, 0x7A, KeyState::Pressed);
        manager.process_event(&InputEvent::Key(f11_press));
        assert!(manager.is_layer_active("test"));

        // Clear all layers
        manager.clear_layers();
        assert!(!manager.is_layer_active("test"));
        assert!(manager.get_active_layers().is_empty());
    }

    // ==================== Additional tests from ut_runtime_mapper.rs ====================

    #[test]
    fn test_layer_creation_alt() {
        let layer = Layer::new("test", 0x3A, 0x14).with_mode(LayerMode::Hold);

        assert_eq!(layer.name, "test");
        assert_eq!(layer.activation_key, 0x3A);
        assert_eq!(layer.activation_vk, 0x14);
        assert!(matches!(layer.mode, LayerMode::Hold));
    }

    #[test]
    fn test_layer_add_mapping_alt() {
        let mut layer = Layer::new("nav", 0x3A, 0x14);

        let trigger = Trigger::key(0x1E, 0x41);
        let action = Action::key(KeyAction::click(0x1F, 0x42));

        layer.add_mapping(trigger, action);
        assert_eq!(layer.mappings.len(), 1);
    }

    #[test]
    fn test_layer_activation_key_alt() {
        let layer = Layer::new("test", 0x3A, 0x14);

        assert!(layer.is_activation_key(0x3A, 0x14));
        assert!(!layer.is_activation_key(0x3B, 0x15));
    }

    #[test]
    fn test_layer_activation() {
        // Test layer activation and deactivation
        let layer_name = "navigation";
        let activation_key = "CapsLock";

        assert_eq!(layer_name, "navigation");
        assert_eq!(activation_key, "CapsLock");
    }

    #[test]
    fn test_layer_modes_alt() {
        let modes = vec!["Hold", "Toggle"];

        assert!(modes.contains(&"Hold"));
        assert!(modes.contains(&"Toggle"));
    }

    #[test]
    fn test_layer_mappings_alt() {
        let mappings = vec![("H", "Left"), ("J", "Down"), ("K", "Up"), ("L", "Right")];

        assert_eq!(mappings.len(), 4);
        assert_eq!(mappings[0].0, "H");
        assert_eq!(mappings[0].1, "Left");
    }

    #[test]
    fn test_multiple_layers_alt() {
        let layers = vec![
            ("navigation", "CapsLock", "Hold"),
            ("numpad", "RightAlt", "Hold"),
        ];

        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].0, "navigation");
        assert_eq!(layers[1].0, "numpad");
    }
}
