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
}
