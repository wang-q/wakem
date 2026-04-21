use crate::types::{
    Action, InputEvent, KeyAction, KeyEvent, KeyState, Layer, LayerMode, LayerStack,
    MappingRule, Trigger,
};
use std::collections::HashMap;
use tracing::{debug, trace};

/// Layer manager
pub struct LayerManager {
    /// All available layers
    layers: HashMap<String, Layer>,
    /// Layer stack (manages activation state)
    stack: LayerStack,
    /// Base layer mappings (simple remapping loaded from config)
    base_mappings: Vec<MappingRule>,
}

impl LayerManager {
    pub fn new() -> Self {
        Self {
            layers: HashMap::new(),
            stack: LayerStack::new(),
            base_mappings: Vec::new(),
        }
    }

    /// Register a layer
    pub fn register_layer(&mut self, layer: Layer) {
        debug!("Registering layer: {}", layer.name);
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
        // Check if it's an activation key for any layer
        for layer in self.layers.values() {
            if layer.is_activation_key(event.scan_code, event.virtual_key) {
                match layer.mode {
                    LayerMode::Hold => {
                        match event.state {
                            KeyState::Pressed => {
                                trace!("Activating layer (Hold): {}", layer.name);
                                if let Some(layer) = self.layers.get(&layer.name).cloned() {
                                    self.stack.hold_layer(&layer.name);
                                    self.stack.activate_layer(layer);
                                }
                            }
                            KeyState::Released => {
                                trace!("Deactivating layer (Hold): {}", layer.name);
                                self.stack.release_layer(&layer.name);
                            }
                        }
                        // Layer activation key itself is not passed through
                        return (true, None);
                    }
                    LayerMode::Toggle => {
                        if event.state == KeyState::Pressed {
                            trace!("Toggling layer: {}", layer.name);
                            if let Some(layer) = self.layers.get(&layer.name).cloned() {
                                self.stack.toggle_layer(layer);
                            }
                            // Toggle key itself is not passed through
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
}
