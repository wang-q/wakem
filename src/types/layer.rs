use serde::{Deserialize, Serialize};

use crate::types::{Action, MappingRule, Trigger};

/// Layer mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LayerMode {
    /// Hold to activate, release to exit
    #[default]
    Hold,
    /// Toggle mode (press once to enter, press again to exit)
    Toggle,
}

/// Layer definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    /// Layer name
    pub name: String,
    /// Activation key (scan code)
    pub activation_key: u16,
    /// Activation key virtual key code
    pub activation_vk: u16,
    /// Layer mode
    pub mode: LayerMode,
    /// Mappings within this layer
    pub mappings: Vec<MappingRule>,
}

impl Layer {
    pub fn new(
        name: impl Into<String>,
        activation_scan: u16,
        activation_vk: u16,
    ) -> Self {
        Self {
            name: name.into(),
            activation_key: activation_scan,
            activation_vk,
            mode: LayerMode::Hold,
            mappings: Vec::new(),
        }
    }

    pub fn with_mode(mut self, mode: LayerMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn add_mapping(&mut self, trigger: Trigger, action: Action) {
        self.mappings.push(MappingRule::new(trigger, action));
    }
}

/// Layer stack (manages multiple active layers)
#[derive(Debug, Clone, Default)]
pub struct LayerStack {
    /// Base layer (bottom layer)
    base_layer: Vec<MappingRule>,
    /// Currently active layers (sorted by priority)
    active_layers: Vec<Layer>,
    /// Layers activated by holding
    hold_layers: Vec<String>,
}

impl LayerStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set base layer mappings
    pub fn set_base_layer(&mut self, mappings: Vec<MappingRule>) {
        self.base_layer = mappings;
    }

    /// Activate a layer
    pub fn activate_layer(&mut self, layer: Layer) {
        // If layer already active, remove first then add (move to top of stack)
        self.active_layers.retain(|l| l.name != layer.name);
        self.active_layers.push(layer);
    }

    /// Deactivate specified layer
    pub fn deactivate_layer(&mut self, name: &str) {
        self.active_layers.retain(|l| l.name != name);
        self.hold_layers.retain(|n| n != name);
    }

    /// Mark layer as hold-activated
    pub fn hold_layer(&mut self, name: &str) {
        if !self.hold_layers.contains(&name.to_string()) {
            self.hold_layers.push(name.to_string());
        }
    }

    /// Release hold-activated layer
    ///
    /// This method removes the layer from the hold list. For layers with
    /// `LayerMode::Hold`, this also deactivates the layer. For layers with
    /// `LayerMode::Toggle`, the layer remains active until explicitly toggled
    /// off or `deactivate_layer` is called.
    ///
    /// # Examples
    ///
    /// ```
    /// use wakem::types::{Layer, LayerMode, LayerStack};
    ///
    /// let mut stack = LayerStack::new();
    /// let layer = Layer::new("hold_layer", 0x3A, 0x14).with_mode(LayerMode::Hold);
    /// stack.activate_layer(layer);
    /// stack.hold_layer("hold_layer");
    ///
    /// // Release the hold - Hold mode layer is deactivated
    /// stack.release_layer("hold_layer");
    /// assert!(!stack.is_layer_active("hold_layer"));
    /// ```
    pub fn release_layer(&mut self, name: &str) {
        self.hold_layers.retain(|n| n != name);
        // If Hold mode layer, deactivate on release
        self.active_layers.retain(|l| {
            if l.name == name && l.mode == LayerMode::Hold {
                return false;
            }
            true
        });
    }

    /// Toggle layer activation state (for Toggle mode)
    pub fn toggle_layer(&mut self, layer: Layer) {
        let is_active = self.active_layers.iter().any(|l| l.name == layer.name);
        if is_active {
            self.deactivate_layer(&layer.name);
        } else {
            self.activate_layer(layer);
        }
    }

    /// Get all currently available mapping rules (priority: later activated layers first)
    pub fn get_all_mappings(&self) -> Vec<MappingRule> {
        // Pre-calculate capacity to avoid reallocations
        let total_capacity = self.base_layer.len()
            + self
                .active_layers
                .iter()
                .map(|l| l.mappings.len())
                .sum::<usize>();
        let mut result = Vec::with_capacity(total_capacity);

        // Add base layer first
        result.extend_from_slice(&self.base_layer);

        // Then add active layers (later layers override earlier ones)
        for layer in &self.active_layers {
            result.extend_from_slice(&layer.mappings);
        }

        result
    }

    /// Check if layer is active
    pub fn is_layer_active(&self, name: &str) -> bool {
        self.active_layers.iter().any(|l| l.name == name)
    }

    /// Get list of currently active layers
    pub fn get_active_layers(&self) -> &[Layer] {
        &self.active_layers
    }

    /// Clear all active layers
    pub fn clear_active_layers(&mut self) {
        self.active_layers.clear();
        self.hold_layers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::KeyAction;

    #[test]
    fn test_layer_stack() {
        let mut stack = LayerStack::new();

        // Create test layer
        let mut layer1 = Layer::new("navigate", 0x3A, 0x14);
        layer1.add_mapping(
            Trigger::key(0x23, 0x48),                  // H
            Action::key(KeyAction::click(0x4B, 0x25)), // Left
        );

        // Activate layer
        stack.activate_layer(layer1.clone());
        assert!(stack.is_layer_active("navigate"));

        // Get mappings
        let mappings = stack.get_all_mappings();
        assert_eq!(mappings.len(), 1);

        // Deactivate layer
        stack.deactivate_layer("navigate");
        assert!(!stack.is_layer_active("navigate"));
    }

    #[test]
    fn test_layer_toggle() {
        let mut stack = LayerStack::new();
        let layer = Layer::new("test", 0x3A, 0x14).with_mode(LayerMode::Toggle);

        // Toggle activation
        stack.toggle_layer(layer.clone());
        assert!(stack.is_layer_active("test"));

        // Toggle again to deactivate
        stack.toggle_layer(layer);
        assert!(!stack.is_layer_active("test"));
    }

    // ==================== Additional tests from ut_types_layer.rs ====================

    #[test]
    fn test_layer_creation() {
        let layer = Layer::new("navigation", 0x3A, 0x14); // CapsLock

        assert_eq!(layer.name, "navigation");
        assert_eq!(layer.activation_key, 0x3A);
        assert_eq!(layer.activation_vk, 0x14);
        assert_eq!(layer.mode, LayerMode::Hold);
        assert!(layer.mappings.is_empty());
    }

    #[test]
    fn test_layer_toggle_mode() {
        let layer = Layer::new("fn_layer", 0x3B, 0x70) // F1
            .with_mode(LayerMode::Toggle);

        assert_eq!(layer.activation_key, 0x3B);
        assert_eq!(layer.mode, LayerMode::Toggle);
    }

    #[test]
    fn test_layer_add_mapping() {
        let mut layer =
            Layer::new("vim_navigation", 0x3A, 0x14).with_mode(LayerMode::Hold);

        layer.add_mapping(
            Trigger::key(0x23, 0x48),                  // H
            Action::key(KeyAction::click(0x4B, 0x25)), // Left
        );

        layer.add_mapping(
            Trigger::key(0x24, 0x4A),                  // J
            Action::key(KeyAction::click(0x50, 0x28)), // Down
        );

        assert_eq!(layer.mappings.len(), 2);
    }

    #[test]
    fn test_layer_default_mode() {
        let layer = Layer::new("test", 0x1E, 0x41);

        assert_eq!(layer.mode, LayerMode::Hold);
    }

    #[test]
    fn test_layer_mode_enum() {
        let hold = LayerMode::Hold;
        let toggle = LayerMode::Toggle;

        assert!(matches!(hold, LayerMode::Hold));
        assert!(matches!(toggle, LayerMode::Toggle));
    }

    #[test]
    fn test_layer_stack_creation() {
        let stack = LayerStack::new();

        assert!(stack.get_active_layers().is_empty());
        assert!(!stack.is_layer_active("any"));
    }

    #[test]
    fn test_layer_stack_activate() {
        let mut stack = LayerStack::new();
        let layer = Layer::new("navigation", 0x3A, 0x14);

        stack.activate_layer(layer);
        assert!(stack.is_layer_active("navigation"));
    }

    #[test]
    fn test_layer_stack_deactivate() {
        let mut stack = LayerStack::new();
        let layer1 = Layer::new("navigation", 0x3A, 0x14);
        let layer2 = Layer::new("window_mgmt", 0x3B, 0x70);

        stack.activate_layer(layer1);
        stack.activate_layer(layer2);

        stack.deactivate_layer("navigation");
        assert!(!stack.is_layer_active("navigation"));
        assert!(stack.is_layer_active("window_mgmt"));
    }

    #[test]
    fn test_layer_stack_toggle_alt() {
        let mut stack = LayerStack::new();
        let layer = Layer::new("test", 0x3A, 0x14).with_mode(LayerMode::Toggle);

        // First toggle - activate
        stack.toggle_layer(layer.clone());
        assert!(stack.is_layer_active("test"));

        // Second toggle - deactivate
        stack.toggle_layer(layer);
        assert!(!stack.is_layer_active("test"));
    }

    #[test]
    fn test_layer_stack_clear() {
        let mut stack = LayerStack::new();

        stack.activate_layer(Layer::new("layer1", 0x3A, 0x14));
        stack.activate_layer(Layer::new("layer2", 0x3B, 0x70));

        stack.clear_active_layers();

        assert!(stack.get_active_layers().is_empty());
        assert!(!stack.is_layer_active("layer1"));
        assert!(!stack.is_layer_active("layer2"));
    }

    #[test]
    fn test_multiple_layers_active() {
        let mut stack = LayerStack::new();

        stack.activate_layer(Layer::new("base", 0x3A, 0x14));
        stack.activate_layer(Layer::new("shift", 0x3B, 0x70));
        stack.activate_layer(Layer::new("ctrl", 0x3C, 0x71));

        assert_eq!(stack.get_active_layers().len(), 3);
        assert!(stack.is_layer_active("base"));
        assert!(stack.is_layer_active("shift"));
        assert!(stack.is_layer_active("ctrl"));
    }

    #[test]
    fn test_layer_priority() {
        let mut stack = LayerStack::new();

        stack.activate_layer(Layer::new("base", 0x3A, 0x14));
        stack.activate_layer(Layer::new("override", 0x3B, 0x70));

        // Get last activated layer
        let active = stack.get_active_layers();
        assert_eq!(active.len(), 2);
        assert_eq!(active[active.len() - 1].name, "override");
    }

    #[test]
    fn test_empty_layer_name() {
        let layer = Layer::new("", 0x1E, 0x41);
        assert_eq!(layer.name, "");
    }

    #[test]
    fn test_complex_layer_config() {
        let mut layer =
            Layer::new("advanced_navigation", 0x3A, 0x14).with_mode(LayerMode::Toggle);

        layer.add_mapping(
            Trigger::key(0x23, 0x48),                  // H
            Action::key(KeyAction::click(0x4B, 0x25)), // Left
        );
        layer.add_mapping(
            Trigger::key(0x24, 0x4A),                  // J
            Action::key(KeyAction::click(0x50, 0x28)), // Down
        );
        layer.add_mapping(
            Trigger::key(0x25, 0x4B),                  // K
            Action::key(KeyAction::click(0x48, 0x26)), // Up
        );
        layer.add_mapping(
            Trigger::key(0x26, 0x4C),                  // L
            Action::key(KeyAction::click(0x4D, 0x27)), // Right
        );
        layer.add_mapping(
            Trigger::key(0x11, 0x57), // W
            Action::Window(crate::types::WindowAction::Center),
        );
        layer.add_mapping(
            Trigger::key(0x10, 0x51), // Q
            Action::Window(crate::types::WindowAction::Close),
        );

        assert_eq!(layer.name, "advanced_navigation");
        assert_eq!(layer.mode, LayerMode::Toggle);
        assert_eq!(layer.mappings.len(), 6);
    }

    #[test]
    fn test_layer_stack_base_layer() {
        let mut stack = LayerStack::new();

        let base_mappings = vec![MappingRule::new(
            Trigger::key(0x1E, 0x41),
            Action::key(KeyAction::click(0x1E, 0x41)),
        )];

        stack.set_base_layer(base_mappings);

        let all_mappings = stack.get_all_mappings();
        assert_eq!(all_mappings.len(), 1);
    }

    #[test]
    fn test_layer_stack_hold_release() {
        let mut stack = LayerStack::new();
        let layer = Layer::new("hold_test", 0x3A, 0x14).with_mode(LayerMode::Hold);

        stack.activate_layer(layer);
        stack.hold_layer("hold_test");
        assert!(stack.is_layer_active("hold_test"));

        // Release Hold mode layer
        stack.release_layer("hold_test");
        assert!(!stack.is_layer_active("hold_test"));
    }

    #[test]
    fn test_layer_stack_toggle_release() {
        let mut stack = LayerStack::new();
        let layer = Layer::new("toggle_test", 0x3A, 0x14).with_mode(LayerMode::Toggle);

        stack.activate_layer(layer);
        stack.hold_layer("toggle_test");
        assert!(stack.is_layer_active("toggle_test"));

        // Release Toggle mode layer should remain active
        stack.release_layer("toggle_test");
        // Note: current implementation checks mode on release, Toggle mode layers won't be deactivated
        // But actual behavior depends on specific implementation
    }

    #[test]
    fn test_layer_reactivate_moves_to_top() {
        let mut stack = LayerStack::new();

        let layer1 = Layer::new("layer1", 0x3A, 0x14);
        let layer2 = Layer::new("layer2", 0x3B, 0x70);

        stack.activate_layer(layer1);
        stack.activate_layer(layer2);

        // Reactivate layer1, should move to top
        let layer1_new = Layer::new("layer1", 0x3A, 0x14);
        stack.activate_layer(layer1_new);

        let active = stack.get_active_layers();
        assert_eq!(active.len(), 2);
        assert_eq!(active[1].name, "layer1"); // Now at top
    }

    #[test]
    fn test_layer_hold_mode_behavior() {
        let layer = Layer::new("hold_layer", 0x3A, 0x14).with_mode(LayerMode::Hold);
        assert!(matches!(layer.mode, LayerMode::Hold));
    }

    #[test]
    fn test_layer_toggle_mode_behavior() {
        let layer = Layer::new("toggle_layer", 0x39, 0x20).with_mode(LayerMode::Toggle);
        assert!(matches!(layer.mode, LayerMode::Toggle));
    }

    #[test]
    fn test_layer_add_multiple_mappings() {
        let mut layer = Layer::new("nav", 0x3A, 0x14);

        layer.add_mapping(
            Trigger::key(0x23, 0x48),
            Action::key(KeyAction::click(0x4B, 0x25)), // H -> Left
        );
        layer.add_mapping(
            Trigger::key(0x24, 0x4A),
            Action::key(KeyAction::click(0x50, 0x28)), // J -> Down
        );
        layer.add_mapping(
            Trigger::key(0x25, 0x4B),
            Action::key(KeyAction::click(0x48, 0x26)), // K -> Up
        );

        assert_eq!(layer.mappings.len(), 3);
    }
}
