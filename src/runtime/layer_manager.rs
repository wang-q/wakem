use crate::types::{
    Action, InputEvent, KeyAction, KeyEvent, KeyState, Layer, LayerMode, LayerStack,
    MappingRule, Trigger,
};
use std::collections::HashMap;
use tracing::debug;

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
        self.stack.set_base_layer(mappings.clone());
        self.base_mappings = mappings;
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
        debug!(
            scan_code = event.scan_code,
            virtual_key = event.virtual_key,
            state = ?event.state,
            layers_count = self.layers.len(),
            activation_keys_count = self.activation_key_index.len(),
            base_mappings_count = self.base_mappings.len(),
            "LayerManager processing key event"
        );

        let lookup_key = ActivationKey {
            scan_code: event.scan_code,
            virtual_key: event.virtual_key,
        };

        if let Some(layer_name) = self.activation_key_index.get(&lookup_key) {
            debug!(layer_name = %layer_name, "Found activation key");
            if let Some(layer) = self.layers.get(layer_name) {
                let layer_name = &layer.name;
                match layer.mode {
                    LayerMode::Hold => {
                        match event.state {
                            KeyState::Pressed => {
                                debug!("Activating layer (Hold): {}", layer_name);
                                self.stack.hold_layer(layer_name);
                                self.stack.activate_layer(layer.clone());
                            }
                            KeyState::Released => {
                                debug!("Deactivating layer (Hold): {}", layer_name);
                                self.stack.release_layer(layer_name);
                            }
                        }
                        return (true, None);
                    }
                    LayerMode::Toggle => {
                        if event.state == KeyState::Pressed {
                            debug!("Toggling layer: {}", layer_name);
                            self.stack.toggle_layer(layer.clone());
                            return (true, None);
                        }
                    }
                }
            }
        }

        // If not an activation key, search for mapping in active layers
        let input_event = InputEvent::Key(event.clone());
        let mappings = self.stack.get_all_mappings();
        debug!(mappings_count = mappings.len(), "Checking layer mappings");
        for (idx, rule) in mappings.iter().enumerate() {
            debug!(rule_idx = idx, trigger = ?rule.trigger, "Checking layer rule");
            if rule.trigger.matches(&input_event) {
                debug!("Found mapping in layer stack: {:?}", rule.action);
                return (true, Some(rule.action.clone()));
            }
        }

        // Also check base mappings directly
        debug!(
            base_mappings_count = self.base_mappings.len(),
            "Checking base mappings"
        );
        for (idx, rule) in self.base_mappings.iter().enumerate() {
            debug!(rule_idx = idx, trigger = ?rule.trigger, "Checking base rule");
            if rule.trigger.matches(&input_event) {
                debug!("Found mapping in base: {:?}", rule.action);
                return (true, Some(rule.action.clone()));
            }
        }

        debug!("No mapping found in LayerManager");
        (false, None)
    }

    /// Create layer from config
    ///
    /// Supports key-to-key, key-to-window-action, and key-to-modifier-combo
    /// mappings, consistent with `config::parse_layer_mappings`.
    pub fn create_layer_from_config(
        name: &str,
        activation_key: &str,
        mode: LayerMode,
        mappings: &[(String, String)],
    ) -> anyhow::Result<Layer> {
        use crate::config::{parse_key, parse_modifier_combo, parse_window_action};

        let (scan, vk) = parse_key(activation_key)?;
        let mut layer = Layer::new(name, scan, vk).with_mode(mode);

        for (from, to) in mappings {
            let from_key = parse_key(from)?;
            let trigger = Trigger::key(from_key.0, from_key.1);

            if let Ok(window_action) = parse_window_action(to) {
                layer.add_mapping(trigger, Action::window(window_action));
            } else if let Ok(modifiers) = parse_modifier_combo(to) {
                let action = crate::config::create_hyper_key_action(&modifiers);
                layer.add_mapping(trigger, action);
            } else {
                let to_key = parse_key(to)?;
                layer.add_mapping(
                    trigger,
                    Action::key(KeyAction::click(to_key.0, to_key.1)),
                );
            }
        }

        Ok(layer)
    }
}

impl Default for LayerManager {
    fn default() -> Self {
        Self::new()
    }
}
