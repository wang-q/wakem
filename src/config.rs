use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

use crate::constants::{
    DEFAULT_ACCELERATION_MULTIPLIER, DEFAULT_WHEEL_SPEED, DEFAULT_WHEEL_STEP,
};
use crate::types::{ContextCondition, MacroStep, MappingRule};

use crate::platform::launcher_common::Launcher;

/// Global configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Whether to show system tray icon
    #[serde(default = "default_true")]
    pub tray_icon: bool,
    /// Whether to auto-reload configuration
    #[serde(default = "default_true")]
    pub auto_reload: bool,
    /// Custom tray icon path
    #[serde(default)]
    pub icon_path: Option<String>,
    /// Keyboard mapping rules
    #[serde(default)]
    pub keyboard: KeyboardConfig,
    /// Window settings
    #[serde(default)]
    pub window: WindowConfig,
    /// Mouse settings
    #[serde(default)]
    pub mouse: MouseConfig,
    /// Launch settings
    #[serde(default)]
    pub launch: HashMap<String, String>,
    /// Network communication settings
    #[serde(default)]
    pub network: NetworkConfig,
    /// Macro definitions: macro_name -> [MacroStep, ...]
    #[serde(default)]
    pub macros: HashMap<String, Vec<MacroStep>>,
    /// Macro trigger key mappings: trigger -> macro_name
    #[serde(default)]
    pub macro_bindings: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            tray_icon: true,
            auto_reload: true,
            icon_path: None,
            keyboard: KeyboardConfig::default(),
            window: WindowConfig::default(),
            mouse: MouseConfig::default(),
            launch: HashMap::new(),
            network: NetworkConfig::default(),
            macros: HashMap::new(),
            macro_bindings: HashMap::new(),
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_str(&content)
    }

    /// Parse configuration from string
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(content: &str) -> anyhow::Result<Self> {
        let config: Config = toml::from_str(content)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration integrity and business rules
    pub fn validate(&self) -> anyhow::Result<()> {
        // 1. Validate log level
        match self.log_level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "warning" | "error" => {}
            other => {
                anyhow::bail!(
                    "Invalid log_level '{}': must be one of trace, debug, info, warn, error",
                    other
                );
            }
        }

        // 2. Validate instance ID range (must be checked before get_instance_port)
        if self.network.instance_id > 255 {
            anyhow::bail!(
                "Invalid instance_id {}: must be in range 0-255",
                self.network.instance_id
            );
        }

        // 3. Validate network port range (u16 max is 65535, only need to check minimum)
        let port = crate::ipc::get_instance_port(self.network.instance_id);
        if port < 1024 {
            anyhow::bail!("Invalid port {}: must be in range 1024-65535", port);
        }

        // 3.5. Validate auth_key is not empty string when explicitly set
        // An empty auth_key would bypass authentication, allowing any local
        // process to control the daemon. Use None (auto-generate) instead.
        if let Some(ref key) = self.network.auth_key {
            if key.is_empty() {
                anyhow::bail!(
                    "Invalid network.auth_key: empty string is not allowed. \
                     Remove auth_key to auto-generate, or set a non-empty value"
                );
            }
        }

        // 4. Validate macro bindings reference existing macros
        for (trigger, macro_name) in &self.macro_bindings {
            if !self.macros.contains_key(macro_name) {
                anyhow::bail!(
                    "Macro binding '{}' references non-existent macro '{}'",
                    trigger,
                    macro_name
                );
            }
        }

        // 5. Validate macro steps are not empty (warning only)
        for (macro_name, steps) in &self.macros {
            if steps.is_empty() {
                tracing::warn!(
                    "Macro '{}' has no steps defined, it will do nothing",
                    macro_name
                );
            }
        }

        // 6. Validate layer activation keys are not empty
        for (layer_name, layer) in &self.keyboard.layers {
            if layer.activation_key.is_empty() {
                anyhow::bail!("Layer '{}' has empty activation_key", layer_name);
            }
        }

        // 7. Validate mouse wheel acceleration_multiplier range
        let multiplier = self.mouse.wheel.acceleration_multiplier;
        if !(0.1..=10.0).contains(&multiplier) {
            anyhow::bail!(
                "Invalid mouse.wheel.acceleration_multiplier: {}. Must be in range 0.1-10.0",
                multiplier
            );
        }

        // 8. Validate wheel speed is positive
        if self.mouse.wheel.speed <= 0 {
            anyhow::bail!(
                "Invalid mouse.wheel.speed: {}. Must be positive",
                self.mouse.wheel.speed
            );
        }

        // 9. Validate icon_path exists if specified
        // Icon path validation is a soft check: missing icons are logged as warnings
        // rather than errors, since a missing icon should not prevent the application
        // from starting. The WAKEM_SKIP_ICON_VALIDATION env var can be used to
        // suppress even the warning (primarily for test environments).
        if let Some(ref icon_path) = self.icon_path {
            let skip_validation = std::env::var("WAKEM_SKIP_ICON_VALIDATION").is_ok();
            if !skip_validation && !std::path::Path::new(icon_path).exists() {
                tracing::warn!(
                    "Icon path '{}' does not exist, using default icon",
                    icon_path
                );
            }
        }

        // 10. Validate launch program paths are not empty
        for (trigger, command) in &self.launch {
            if command.trim().is_empty() {
                anyhow::bail!("Launch command for trigger '{}' is empty", trigger);
            }
        }

        // 11. Validate keyboard.remap keys are valid
        for (from, to) in &self.keyboard.remap {
            if let Err(e) = parse_key(from) {
                anyhow::bail!("Invalid key '{}' in keyboard.remap: {}", from, e);
            }
            // Try to parse as key, window action, or modifier combo (e.g., "Ctrl+Alt+Win" or just "Ctrl")
            let is_valid_target = parse_key(to).is_ok()
                || parse_window_action(to).is_ok()
                || parse_modifier_combo(to).is_ok();
            if !is_valid_target {
                anyhow::bail!(
                    "Invalid target '{}' in keyboard.remap for key '{}': must be a valid key, window action, or modifier combo",
                    to, from
                );
            }
        }

        // 12. Validate window.shortcuts
        for (shortcut, action) in &self.window.shortcuts {
            if let Err(e) = parse_shortcut_trigger(shortcut) {
                anyhow::bail!(
                    "Invalid shortcut '{}' in window.shortcuts: {}",
                    shortcut,
                    e
                );
            }
            if let Err(e) = parse_window_action(action) {
                anyhow::bail!(
                    "Invalid window action '{}' in window.shortcuts: {}",
                    action,
                    e
                );
            }
        }

        // 13. Validate keyboard.layers mappings are parseable
        for (layer_name, layer) in &self.keyboard.layers {
            if let Err(e) = parse_key(&layer.activation_key) {
                anyhow::bail!(
                    "Invalid activation_key '{}' in keyboard.layers.{}: {}",
                    layer.activation_key,
                    layer_name,
                    e
                );
            }
            for (from, to) in &layer.mappings {
                if let Err(e) = parse_key(from) {
                    anyhow::bail!(
                        "Invalid key '{}' in keyboard.layers.{}.mappings: {}",
                        from,
                        layer_name,
                        e
                    );
                }
                let is_valid_target = parse_key(to).is_ok()
                    || parse_window_action(to).is_ok()
                    || (to.contains('+') && parse_modifier_combo(to).is_ok())
                    || (to.contains('+') && parse_shortcut_trigger(to).is_ok());
                if !is_valid_target {
                    anyhow::bail!(
                        "Invalid target '{}' in keyboard.layers.{}.mappings for key '{}': must be a valid key, modifier combo, shortcut, or window action",
                        to, layer_name, from
                    );
                }
            }
        }

        // 14. Validate launch trigger keys are parseable
        for trigger in self.launch.keys() {
            if let Err(e) = parse_shortcut_trigger(trigger) {
                anyhow::bail!("Invalid trigger '{}' in launch: {}", trigger, e);
            }
        }

        // 15. Validate context_mappings are parseable
        for (idx, ctx_mapping) in self.keyboard.context_mappings.iter().enumerate() {
            for (from, to) in &ctx_mapping.mappings {
                if let Err(e) = parse_key(from) {
                    anyhow::bail!(
                        "Invalid key '{}' in keyboard.context_mappings[{}].mappings: {}",
                        from,
                        idx,
                        e
                    );
                }
                let is_valid_target = parse_key(to).is_ok()
                    || parse_window_action(to).is_ok()
                    || (to.contains('+') && parse_modifier_combo(to).is_ok())
                    || (to.contains('+') && parse_shortcut_trigger(to).is_ok());
                if !is_valid_target {
                    anyhow::bail!(
                        "Invalid target '{}' in keyboard.context_mappings[{}].mappings for key '{}': must be a valid key, modifier combo, shortcut, or window action",
                        to, idx, from
                    );
                }
            }
        }

        Ok(())
    }

    /// Validate macro name
    /// Rules:
    /// - Length: 1-50 characters
    /// - Allowed characters: alphanumeric, underscore, hyphen
    pub fn validate_macro_name(name: &str) -> anyhow::Result<()> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Macro name cannot be empty"));
        }

        if name.len() > 50 {
            return Err(anyhow::anyhow!("Macro name too long (max 50 characters)"));
        }

        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(anyhow::anyhow!(
                "Macro name contains invalid characters. Only alphanumeric, underscore, and hyphen are allowed"
            ));
        }

        Ok(())
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get all mapping rules
    pub fn get_all_rules(&self) -> Vec<MappingRule> {
        let mut rules = Vec::new();

        for (k, v) in &self.keyboard.remap {
            match parse_key_mapping(k, v) {
                Ok(rule) => rules.push(rule),
                Err(e) => tracing::warn!("Skipping keyboard.remap '{}': {}", k, e),
            }
        }

        for (name, layer) in &self.keyboard.layers {
            match self.parse_layer_mappings(name, layer) {
                Ok(layer_rules) => rules.extend(layer_rules),
                Err(e) => tracing::warn!("Skipping keyboard.layers '{}': {}", name, e),
            }
        }

        for (k, v) in &self.window.shortcuts {
            match parse_window_shortcut(k, v) {
                Ok(rule) => rules.push(rule),
                Err(e) => tracing::warn!("Skipping window.shortcuts '{}': {}", k, e),
            }
        }

        for (k, v) in &self.launch {
            match parse_launch_mapping(k, v) {
                Ok(rule) => rules.push(rule),
                Err(e) => tracing::warn!("Skipping launch '{}': {}", k, e),
            }
        }

        rules
    }

    /// Extract hyper key mappings from keyboard remap configuration
    /// Returns a map of (scan_code, virtual_key) -> ModifierState for each key
    /// that is remapped to a modifier combination (e.g., CapsLock = "Ctrl+Alt+Meta")
    pub fn get_hyper_key_mappings(
        &self,
    ) -> std::collections::HashMap<(u16, u16), crate::types::ModifierState> {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        for (key_str, target_str) in &self.keyboard.remap {
            if target_str.contains('+') && !target_str.contains("->") {
                if let Ok((sc, vk)) = parse_key(key_str) {
                    if let Ok(modifiers) = parse_modifier_combo(target_str) {
                        map.insert((sc, vk), modifiers);
                        debug!(
                            scan_code = sc,
                            virtual_key = vk,
                            ?modifiers,
                            "Found hyper key mapping"
                        );
                    }
                }
            }
        }
        map
    }

    /// Parse layer mapping rules
    fn parse_layer_mappings(
        &self,
        _layer_name: &str,
        layer: &LayerConfig,
    ) -> anyhow::Result<Vec<MappingRule>> {
        use crate::types::{Action, KeyAction, Trigger};

        let mut rules = Vec::new();

        // Parse activation key
        let activation_key = parse_key(&layer.activation_key)?;
        let activation_trigger = Trigger::key(activation_key.0, activation_key.1);

        // Create layer switch action based on mode
        let layer_action = match layer.mode {
            LayerMode::Hold => Action::key(KeyAction::Press {
                scan_code: activation_key.0,
                virtual_key: activation_key.1,
            }),
            LayerMode::Toggle => Action::key(KeyAction::Click {
                scan_code: activation_key.0,
                virtual_key: activation_key.1,
            }),
        };

        // Add layer activation rule
        rules.push(MappingRule::new(activation_trigger, layer_action));

        // Parse mappings within layer
        for (from, to) in &layer.mappings {
            if let Ok(from_key) = parse_key(from) {
                // Check if it's a window management action
                if let Ok(window_action) = parse_window_action(to) {
                    let trigger = Trigger::key(from_key.0, from_key.1);
                    let action = Action::window(window_action);
                    rules.push(MappingRule::new(trigger, action));
                } else if let Ok(to_key) = parse_key(to) {
                    let trigger = Trigger::key(from_key.0, from_key.1);
                    let action = Action::key(KeyAction::click(to_key.0, to_key.1));
                    rules.push(MappingRule::new(trigger, action));
                }
            }
        }

        Ok(rules)
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}

/// Keyboard configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeyboardConfig {
    /// Key remapping (simple mapping)
    #[serde(default)]
    pub remap: HashMap<String, String>,
    /// Shortcut layers
    #[serde(default)]
    pub layers: HashMap<String, LayerConfig>,
    /// Context-aware mappings
    #[serde(default)]
    pub context_mappings: Vec<ContextMapping>,
}

/// Layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfig {
    /// Layer activation key
    pub activation_key: String,
    /// Mappings within layer
    pub mappings: HashMap<String, String>,
    /// Layer activation mode
    #[serde(default)]
    pub mode: LayerMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LayerMode {
    /// Hold to activate, release to exit
    #[default]
    Hold,
    /// Toggle mode (press once to enter, press again to exit)
    Toggle,
}

/// Context-aware mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMapping {
    /// Context condition
    pub context: ContextCondition,
    /// Mapping rules under this context
    pub mappings: HashMap<String, String>,
}

/// Network communication configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkConfig {
    /// Whether to enable network communication
    #[serde(default)]
    pub enabled: bool,
    /// Instance ID (determines port: 57427 + instance_id)
    #[serde(default)]
    pub instance_id: u32,
    /// Pre-shared key
    #[serde(default)]
    pub auth_key: Option<String>,
}

impl NetworkConfig {
    /// Get bind address
    pub fn get_bind_address(&self) -> String {
        crate::ipc::get_instance_address(self.instance_id)
    }

    /// Ensure authentication key exists, generate random key if not
    pub fn ensure_auth_key(&mut self) -> &str {
        if self.auth_key.is_none() {
            let key = Self::generate_random_key();
            debug!("Authentication key generated for security");
            self.auth_key = Some(key);
        }
        self.auth_key.as_deref().unwrap()
    }

    /// Generate random authentication key (32 character hex)
    fn generate_random_key() -> String {
        use rand::Rng;
        let mut rng = rand::rngs::OsRng;
        let bytes: [u8; 16] = rng.gen();
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowConfig {
    /// Window switching settings
    #[serde(default)]
    pub switch: WindowSwitchConfig,
    /// Window position presets (deprecated, kept for backward compatibility)
    #[serde(default)]
    pub positions: HashMap<String, WindowPosition>,
    /// Window management shortcuts (inspired by mrw)
    #[serde(default)]
    pub shortcuts: HashMap<String, String>,
    /// Window preset list
    #[serde(default)]
    pub presets: Vec<WindowPreset>,
    /// Whether to auto-apply presets
    #[serde(default = "default_true")]
    pub auto_apply_preset: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowSwitchConfig {
    /// Whether to ignore minimized windows
    #[serde(default = "default_true")]
    pub ignore_minimal: bool,
    /// Whether to only switch on current virtual desktop
    #[serde(default = "default_true")]
    pub only_current_desktop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Window preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowPreset {
    /// Preset name
    pub name: String,
    /// Matching process name (e.g., chrome.exe)
    #[serde(default)]
    pub process_name: Option<String>,
    /// Matching executable path
    #[serde(default)]
    pub executable_path: Option<String>,
    /// Window title matching pattern (supports wildcards)
    #[serde(default)]
    pub title_pattern: Option<String>,
    /// Window position
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Public wildcard matching function - delegates to types::mapping
pub fn wildcard_match(text: &str, pattern: &str) -> bool {
    crate::types::mapping::wildcard_match(text, pattern)
}

/// Mouse configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MouseConfig {
    /// Button remapping
    #[serde(default)]
    pub button_remap: HashMap<String, String>,
    /// Wheel settings
    #[serde(default)]
    pub wheel: WheelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelConfig {
    /// Wheel speed
    #[serde(default = "default_wheel_speed")]
    pub speed: i32,
    /// Whether to invert wheel direction
    #[serde(default)]
    pub invert: bool,
    /// Whether to enable wheel acceleration
    #[serde(default)]
    pub acceleration: bool,
    /// Acceleration multiplier
    #[serde(default = "default_acceleration_multiplier")]
    pub acceleration_multiplier: f32,
    /// Horizontal scroll configuration
    #[serde(default)]
    pub horizontal_scroll: Option<WheelModifierConfig>,
}

impl Default for WheelConfig {
    fn default() -> Self {
        Self {
            speed: default_wheel_speed(),
            invert: false,
            acceleration: false,
            acceleration_multiplier: default_acceleration_multiplier(),
            horizontal_scroll: None,
        }
    }
}

fn default_wheel_speed() -> i32 {
    DEFAULT_WHEEL_SPEED
}

fn default_acceleration_multiplier() -> f32 {
    DEFAULT_ACCELERATION_MULTIPLIER
}

/// Wheel modifier configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelModifierConfig {
    /// Modifier key (e.g., "Shift", "RightAlt", "RightCtrl")
    pub modifier: String,
    /// Step value for each scroll
    #[serde(default = "default_wheel_step")]
    pub step: i32,
}

fn default_wheel_step() -> i32 {
    DEFAULT_WHEEL_STEP
}

/// Parse simple key mapping configuration
/// Format: "CapsLock" -> "Backspace"
/// Format: "CapsLock" -> "Ctrl+Alt+Win" (mapped as modifier key combination)
fn parse_key_mapping(from: &str, to: &str) -> anyhow::Result<MappingRule> {
    use crate::types::{Action, KeyAction, Trigger};

    let from_key = parse_key(from)?;

    // Check if it's a window management action
    if let Ok(window_action) = parse_window_action(to) {
        let trigger = Trigger::key(from_key.0, from_key.1);
        let action = Action::window(window_action);
        return Ok(MappingRule::new(trigger, action));
    }

    // Check if target is a shortcut with modifier keys (e.g., "Ctrl+Alt+Win")
    // This is used for Hyper key mapping (e.g., CapsLock = "Ctrl+Alt+Win")
    if to.contains('+') && !to.contains("->") {
        // Parse modifier key combination (e.g., "Ctrl+Alt+Win")
        let modifiers = parse_modifier_combo(to)?;
        // Create a special Hyper key action that holds modifiers while key is held
        let action = create_hyper_key_action(&modifiers);
        return Ok(MappingRule::new(
            Trigger::key(from_key.0, from_key.1),
            action,
        ));
    }

    let to_key = parse_key(to)?;

    let trigger = Trigger::key(from_key.0, from_key.1);
    let action = Action::key(KeyAction::click(to_key.0, to_key.1));

    Ok(MappingRule::new(trigger, action))
}

/// Parse pure modifier key combination (e.g., "Ctrl+Alt+Win")
fn parse_modifier_combo(s: &str) -> anyhow::Result<crate::types::ModifierState> {
    use crate::types::ModifierState;

    let mut modifiers = ModifierState::default();
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers.ctrl = true,
            "alt" => modifiers.alt = true,
            "shift" => modifiers.shift = true,
            "win" | "meta" | "command" => modifiers.meta = true,
            _ => {
                // If not a known modifier key, return error
                return Err(anyhow::anyhow!("Unknown modifier: {}", part));
            }
        }
    }

    Ok(modifiers)
}

/// Create Hyper key action
/// When the Hyper key (e.g., CapsLock) is held, it simulates holding the modifier keys
/// This allows using CapsLock+C to trigger Ctrl+Alt+Win+C shortcuts
fn create_hyper_key_action(
    modifiers: &crate::types::ModifierState,
) -> crate::types::Action {
    use crate::types::{Action, KeyAction};

    let mut press_actions = Vec::new();
    let mut release_actions = Vec::new();

    let modifier_keys = [
        ("lctrl", modifiers.ctrl),
        ("lalt", modifiers.alt),
        ("lwin", modifiers.meta),
        ("lshift", modifiers.shift),
    ];

    for (key_name, active) in modifier_keys {
        if active {
            if let Ok((sc, vk)) = parse_key(key_name) {
                press_actions.push(Action::key(KeyAction::press(sc, vk)));
                release_actions.insert(0, Action::key(KeyAction::release(sc, vk)));
            }
        }
    }

    let mut all_actions = press_actions;
    all_actions.push(Action::Delay { milliseconds: 10 });
    all_actions.push(Action::None);
    all_actions.extend(release_actions);

    Action::Sequence(all_actions)
}

/// Parse window management shortcut
/// Format: "Ctrl+Alt+C" -> "Center"
fn parse_window_shortcut(from: &str, to: &str) -> anyhow::Result<MappingRule> {
    use crate::types::Action;

    // Parse shortcut (e.g., "Ctrl+Alt+C")
    let trigger = parse_shortcut_trigger(from)?;

    // Parse window management action
    let window_action = parse_window_action(to)?;
    let action = Action::window(window_action);

    Ok(MappingRule::new(trigger, action))
}

/// Parse shortcut trigger
/// Format: "Ctrl+Alt+C", "Ctrl+Alt+Win+Left"
fn parse_shortcut_trigger(shortcut: &str) -> anyhow::Result<crate::types::Trigger> {
    use crate::types::{ModifierState, Trigger};

    let parts: Vec<&str> = shortcut.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty shortcut"));
    }

    let mut modifiers = ModifierState::new();
    let mut key_name = "";

    for part in &parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers.ctrl = true,
            "alt" => modifiers.alt = true,
            "shift" => modifiers.shift = true,
            "win" | "meta" | "command" | "cmd" => modifiers.meta = true,
            _ => key_name = part,
        }
    }

    if key_name.is_empty() {
        return Err(anyhow::anyhow!(
            "No key specified in shortcut: {}",
            shortcut
        ));
    }

    let key = parse_key(key_name)?;
    Ok(Trigger::key_with_modifiers(key.0, key.1, modifiers))
}

/// Parse window management action
/// Format: "Center", "MoveToEdge(Left)", "HalfScreen(Right)", "FixedRatio(1.333, 0)"
pub fn parse_window_action(
    action_str: &str,
) -> anyhow::Result<crate::types::WindowAction> {
    use crate::types::WindowAction;

    let action_str = action_str.trim();

    // Simple actions (no parameters)
    match action_str {
        "Center" => return Ok(WindowAction::Center),
        "SwitchToNextWindow" => return Ok(WindowAction::SwitchToNextWindow),
        "Minimize" => return Ok(WindowAction::Minimize),
        "Maximize" => return Ok(WindowAction::Maximize),
        "Restore" => return Ok(WindowAction::Restore),
        "Close" => return Ok(WindowAction::Close),
        "ToggleTopmost" => return Ok(WindowAction::ToggleTopmost),
        "ShowDebugInfo" => return Ok(WindowAction::ShowDebugInfo),
        "ApplyPreset" => return Ok(WindowAction::ApplyPreset),
        _ => {}
    }

    // Actions with parameters
    if let Some((name, params)) = action_str.split_once('(') {
        let params = params.trim_end_matches(')');
        let param_list: Vec<&str> = params.split(',').map(|s| s.trim()).collect();

        match name.trim() {
            "MoveToEdge" => {
                let edge_str = param_list
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("MoveToEdge requires an edge parameter (Left, Right, Top, Bottom)"))?;
                if edge_str.is_empty() {
                    return Err(anyhow::anyhow!(
                        "MoveToEdge edge parameter cannot be empty"
                    ));
                }
                let edge = parse_edge(edge_str)?;
                Ok(WindowAction::MoveToEdge(edge))
            }
            "HalfScreen" => {
                let edge_str = param_list
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("HalfScreen requires an edge parameter (Left, Right, Top, Bottom)"))?;
                if edge_str.is_empty() {
                    return Err(anyhow::anyhow!(
                        "HalfScreen edge parameter cannot be empty"
                    ));
                }
                let edge = parse_edge(edge_str)?;
                Ok(WindowAction::HalfScreen(edge))
            }
            "LoopWidth" => {
                let align_str = param_list
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("LoopWidth requires an alignment parameter (Left, Right, Center)"))?;
                if align_str.is_empty() {
                    return Err(anyhow::anyhow!(
                        "LoopWidth alignment parameter cannot be empty"
                    ));
                }
                let align = parse_alignment(align_str)?;
                Ok(WindowAction::LoopWidth(align))
            }
            "LoopHeight" => {
                let align_str = param_list
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("LoopHeight requires an alignment parameter (Top, Bottom, Center)"))?;
                if align_str.is_empty() {
                    return Err(anyhow::anyhow!(
                        "LoopHeight alignment parameter cannot be empty"
                    ));
                }
                let align = parse_alignment(align_str)?;
                Ok(WindowAction::LoopHeight(align))
            }
            "FixedRatio" => {
                let ratio_str = param_list.first().ok_or_else(|| {
                    anyhow::anyhow!(
                        "FixedRatio requires a ratio parameter (e.g., 1.333)"
                    )
                })?;
                let ratio = ratio_str.parse::<f32>()?;
                let scale_index = param_list.get(1).unwrap_or(&"0").parse::<usize>()?;
                Ok(WindowAction::FixedRatio { ratio, scale_index })
            }
            "NativeRatio" => {
                let scale_index = param_list.first().unwrap_or(&"0").parse::<usize>()?;
                Ok(WindowAction::NativeRatio { scale_index })
            }
            "MoveToMonitor" => {
                let direction_str = param_list
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("MoveToMonitor requires a direction parameter (Next, Prev, or index)"))?;
                if direction_str.is_empty() {
                    return Err(anyhow::anyhow!(
                        "MoveToMonitor direction parameter cannot be empty"
                    ));
                }
                let direction = parse_monitor_direction(direction_str)?;
                Ok(WindowAction::MoveToMonitor(direction))
            }
            "Move" => {
                let x_str = param_list.first().ok_or_else(|| {
                    anyhow::anyhow!("Move requires x and y parameters")
                })?;
                let y_str = param_list
                    .get(1)
                    .ok_or_else(|| anyhow::anyhow!("Move requires y parameter"))?;
                let x = x_str.parse::<i32>()?;
                let y = y_str.parse::<i32>()?;
                Ok(WindowAction::Move { x, y })
            }
            "Resize" => {
                let width_str = param_list.first().ok_or_else(|| {
                    anyhow::anyhow!("Resize requires width and height parameters")
                })?;
                let height_str = param_list.get(1).ok_or_else(|| {
                    anyhow::anyhow!("Resize requires height parameter")
                })?;
                let width = width_str.parse::<i32>()?;
                let height = height_str.parse::<i32>()?;
                Ok(WindowAction::Resize { width, height })
            }
            "ShowNotification" => {
                let title = param_list
                    .first()
                    .ok_or_else(|| {
                        anyhow::anyhow!("ShowNotification requires a title parameter")
                    })?
                    .to_string();
                let message = param_list.get(1).unwrap_or(&"").to_string();
                Ok(WindowAction::ShowNotification { title, message })
            }
            "SavePreset" => {
                let name = param_list
                    .first()
                    .ok_or_else(|| {
                        anyhow::anyhow!("SavePreset requires a name parameter")
                    })?
                    .to_string();
                Ok(WindowAction::SavePreset { name })
            }
            "LoadPreset" => {
                let name = param_list
                    .first()
                    .ok_or_else(|| {
                        anyhow::anyhow!("LoadPreset requires a name parameter")
                    })?
                    .to_string();
                Ok(WindowAction::LoadPreset { name })
            }
            _ => Err(anyhow::anyhow!("Unknown window action: {}", name)),
        }
    } else {
        Err(anyhow::anyhow!(
            "Invalid window action format: {}",
            action_str
        ))
    }
}

/// Parse edge parameter
fn parse_edge(s: &str) -> anyhow::Result<crate::types::Edge> {
    use crate::types::Edge;

    match s.trim().to_lowercase().as_str() {
        "left" => Ok(Edge::Left),
        "right" => Ok(Edge::Right),
        "top" => Ok(Edge::Top),
        "bottom" => Ok(Edge::Bottom),
        _ => Err(anyhow::anyhow!("Unknown edge: {}", s)),
    }
}

/// Parse alignment parameter
fn parse_alignment(s: &str) -> anyhow::Result<crate::types::Alignment> {
    use crate::types::Alignment;

    match s.trim().to_lowercase().as_str() {
        "left" => Ok(Alignment::Left),
        "right" => Ok(Alignment::Right),
        "top" => Ok(Alignment::Top),
        "bottom" => Ok(Alignment::Bottom),
        "center" => Ok(Alignment::Center),
        _ => Err(anyhow::anyhow!("Unknown alignment: {}", s)),
    }
}

/// Parse monitor direction parameter
fn parse_monitor_direction(s: &str) -> anyhow::Result<crate::types::MonitorDirection> {
    use crate::types::MonitorDirection;

    match s.trim().to_lowercase().as_str() {
        "next" => Ok(MonitorDirection::Next),
        "prev" | "previous" => Ok(MonitorDirection::Prev),
        s => {
            if let Ok(index) = s.parse::<i32>() {
                Ok(MonitorDirection::Index(index))
            } else {
                Err(anyhow::anyhow!("Unknown monitor direction: {}", s))
            }
        }
    }
}

/// Parse key name to scan code and virtual key code
/// Delegates to the keycode-crate-based implementation in types::key_codes
pub fn parse_key(name: &str) -> anyhow::Result<(u16, u16)> {
    crate::types::key_codes::parse_key(name)
}

/// Config file path cache (reduces repeated file system I/O)
///
/// Performance optimization: caches resolved config file paths to avoid checking file existence on every call
struct ConfigPathCache {
    cache: Mutex<HashMap<u32, Option<std::path::PathBuf>>>,
}

impl ConfigPathCache {
    fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn get_or_resolve(&self, instance_id: u32) -> Option<std::path::PathBuf> {
        // Check cache first (parking_lot::Mutex::lock() returns MutexGuard directly, not Result)
        let mut cache = self.cache.lock();
        if let Some(cached) = cache.get(&instance_id) {
            debug!("Config path cache hit for instance {}", instance_id);
            return cached.clone();
        }

        // Cache miss, resolve path
        let path = Self::resolve_config_path_internal(instance_id);

        // Store in cache
        cache.insert(instance_id, path.clone());

        debug!(
            "Config path cache miss for instance {}, resolved and cached",
            instance_id
        );
        path
    }

    /// Internal path resolution logic (unified across all platforms)
    fn resolve_config_path_internal(instance_id: u32) -> Option<std::path::PathBuf> {
        let config_dir = dirs::config_dir()?;
        let wakem_dir = config_dir.join("wakem");
        let filename = if instance_id == 0 {
            "config.toml".to_string()
        } else {
            format!("config-instance{}.toml", instance_id)
        };
        Some(wakem_dir.join(filename))
    }
}

/// Global config path cache instance
static CONFIG_PATH_CACHE: Lazy<ConfigPathCache> = Lazy::new(ConfigPathCache::new);

/// Resolve config file path (with caching)
///
/// If a path is provided, use it; otherwise use default path (with caching)
/// Supports instance config files (uses config-instanceN.toml when instance_id > 0)
pub fn resolve_config_file_path(
    path: Option<&std::path::Path>,
    instance_id: u32,
) -> Option<std::path::PathBuf> {
    // If explicit path provided, use directly (not cached)
    if let Some(p) = path {
        return Some(p.to_path_buf());
    }

    // Use cached path resolution
    CONFIG_PATH_CACHE.get_or_resolve(instance_id)
}

/// Parse launch item mapping
/// Supported formats:
/// - Simple trigger key: "F1" = "notepad.exe"
/// - Trigger with modifiers: "Ctrl+Alt+Meta+T" = "wt.exe"
/// - Command with arguments: "Ctrl+Alt+Meta+N" = "notepad.exe C:\\Users\\test.txt"
fn parse_launch_mapping(trigger: &str, command: &str) -> anyhow::Result<MappingRule> {
    use crate::types::Action;

    // Parse trigger shortcut (supports modifiers like "Ctrl+Alt+Meta+T")
    let trigger_obj = parse_shortcut_trigger(trigger)?;

    // Parse launch command
    let action = if command.contains(' ') {
        // Use Launcher::parse_command to parse commands with arguments
        Action::Launch(Launcher::parse_command(command))
    } else {
        // Simple command
        Action::launch(command)
    };

    Ok(MappingRule::new(trigger_obj, action))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parse() {
        let config_str = r#"
log_level = "debug"
tray_icon = true
auto_reload = true

[keyboard.remap]
CapsLock = "Backspace"

[keyboard.layers.navigate]
activation_key = "CapsLock"
mode = "Hold"

[keyboard.layers.navigate.mappings]
H = "Left"
J = "Down"
"#;

        let config: Config = toml::from_str(config_str).unwrap();
        assert_eq!(config.log_level, "debug");
        assert!(config.tray_icon);
        assert!(config.keyboard.remap.contains_key("CapsLock"));
        assert!(config.keyboard.layers.contains_key("navigate"));
    }

    #[test]
    fn test_parse_key() {
        assert_eq!(parse_key("capslock").unwrap(), (0x3A, 0x14));
        assert_eq!(parse_key("a").unwrap(), (0x1E, 0x41));
        assert_eq!(parse_key("1").unwrap(), (0x02, 0x31));
        // Test Grave/Backtick key (Alt+` shortcut)
        assert_eq!(parse_key("grave").unwrap(), (0x29, 0xC0));
        assert_eq!(parse_key("backtick").unwrap(), (0x29, 0xC0));
    }

    #[test]
    fn test_parse_key_mapping_with_modifiers() {
        let rule = parse_key_mapping("CapsLock", "Ctrl+Alt+Win").unwrap();

        if let crate::types::Trigger::Key {
            scan_code,
            virtual_key,
            ..
        } = &rule.trigger
        {
            assert_eq!(*scan_code, Some(0x3A));
            assert_eq!(*virtual_key, Some(0x14));
        } else {
            panic!("Expected Key trigger");
        }

        if let crate::types::Action::Sequence(actions) = &rule.action {
            assert_eq!(actions.len(), 8);

            if let crate::types::Action::Key(crate::types::KeyAction::Press {
                virtual_key,
                ..
            }) = &actions[0]
            {
                assert_eq!(*virtual_key, 0xA2); // VK_LCONTROL
            } else {
                panic!("Expected Ctrl Press as first action, got {:?}", actions[0]);
            }

            if let crate::types::Action::Key(crate::types::KeyAction::Press {
                virtual_key,
                ..
            }) = &actions[1]
            {
                assert_eq!(*virtual_key, 0xA4); // VK_LMENU (Alt)
            } else {
                panic!("Expected Alt Press as second action, got {:?}", actions[1]);
            }

            if let crate::types::Action::Key(crate::types::KeyAction::Press {
                virtual_key,
                ..
            }) = &actions[2]
            {
                assert_eq!(*virtual_key, 0x5B); // VK_LWIN
            } else {
                panic!("Expected Win Press as third action, got {:?}", actions[2]);
            }

            if let crate::types::Action::Delay { milliseconds } = &actions[3] {
                assert_eq!(*milliseconds, 10);
            } else {
                panic!("Expected Delay as fourth action, got {:?}", actions[3]);
            }

            assert!(
                matches!(actions[4], crate::types::Action::None),
                "Expected None marker as fifth action, got {:?}",
                actions[4]
            );

            if let crate::types::Action::Key(crate::types::KeyAction::Release {
                virtual_key,
                ..
            }) = &actions[5]
            {
                assert_eq!(*virtual_key, 0x5B); // VK_LWIN release
            } else {
                panic!("Expected Win Release as sixth action, got {:?}", actions[5]);
            }
        } else {
            panic!(
                "Expected Sequence action for modifier combo, got {:?}",
                rule.action
            );
        }
    }

    #[test]
    fn test_parse_modifier_combo() {
        // Test parsing modifier key combination
        let modifiers = parse_modifier_combo("Ctrl+Alt+Win").unwrap();
        assert!(modifiers.ctrl);
        assert!(modifiers.alt);
        assert!(modifiers.meta);
        assert!(!modifiers.shift);

        // Test different order
        let modifiers = parse_modifier_combo("Shift+Ctrl").unwrap();
        assert!(modifiers.ctrl);
        assert!(!modifiers.alt);
        assert!(!modifiers.meta);
        assert!(modifiers.shift);

        // Test case insensitivity
        let modifiers = parse_modifier_combo("ctrl+ALT+win").unwrap();
        assert!(modifiers.ctrl);
        assert!(modifiers.alt);
        assert!(modifiers.meta);
    }

    #[test]
    fn test_parse_window_action_debug() {
        use crate::types::WindowAction;

        // Test ShowDebugInfo
        let action = parse_window_action("ShowDebugInfo").unwrap();
        assert!(matches!(action, WindowAction::ShowDebugInfo));

        // Test ShowNotification
        let action =
            parse_window_action("ShowNotification(wakem, Hello World!)").unwrap();
        if let WindowAction::ShowNotification { title, message } = action {
            assert_eq!(title, "wakem");
            assert_eq!(message, "Hello World!");
        } else {
            panic!("Expected ShowNotification action");
        }

        // Test ShowNotification with default values
        let action = parse_window_action("ShowNotification(Test)").unwrap();
        if let WindowAction::ShowNotification { title, message } = action {
            assert_eq!(title, "Test");
            assert_eq!(message, "");
        } else {
            panic!("Expected ShowNotification action");
        }
    }

    #[test]
    fn test_parse_window_action_center() {
        use crate::types::WindowAction;

        let action = parse_window_action("Center").unwrap();
        assert!(matches!(action, WindowAction::Center));
    }

    #[test]
    fn test_parse_window_action_move_to_edge() {
        use crate::types::{Edge, WindowAction};

        let action = parse_window_action("MoveToEdge(Left)").unwrap();
        assert!(matches!(action, WindowAction::MoveToEdge(Edge::Left)));

        let action = parse_window_action("MoveToEdge(Right)").unwrap();
        assert!(matches!(action, WindowAction::MoveToEdge(Edge::Right)));
    }

    #[test]
    fn test_parse_window_action_half_screen() {
        use crate::types::{Edge, WindowAction};

        let action = parse_window_action("HalfScreen(Left)").unwrap();
        assert!(matches!(action, WindowAction::HalfScreen(Edge::Left)));
    }

    #[test]
    fn test_parse_window_action_loop_width() {
        use crate::types::{Alignment, WindowAction};

        let action = parse_window_action("LoopWidth(Left)").unwrap();
        assert!(matches!(action, WindowAction::LoopWidth(Alignment::Left)));

        let action = parse_window_action("LoopWidth(Right)").unwrap();
        assert!(matches!(action, WindowAction::LoopWidth(Alignment::Right)));
    }

    #[test]
    fn test_parse_window_action_fixed_ratio() {
        use crate::types::WindowAction;

        let action = parse_window_action("FixedRatio(1.333, 0)").unwrap();
        if let WindowAction::FixedRatio { ratio, scale_index } = action {
            assert!((ratio - 1.333).abs() < 0.001);
            assert_eq!(scale_index, 0);
        } else {
            panic!("Expected FixedRatio action");
        }
    }

    #[test]
    fn test_parse_window_action_minimize_maximize() {
        use crate::types::WindowAction;

        let action = parse_window_action("Minimize").unwrap();
        assert!(matches!(action, WindowAction::Minimize));

        let action = parse_window_action("Maximize").unwrap();
        assert!(matches!(action, WindowAction::Maximize));
    }

    #[test]
    fn test_parse_window_action_invalid() {
        // Test invalid actions
        let result = parse_window_action("InvalidAction");
        assert!(result.is_err());

        let result = parse_window_action("MoveToEdge(InvalidEdge)");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_edge() {
        use crate::types::Edge;

        assert!(matches!(parse_edge("left").unwrap(), Edge::Left));
        assert!(matches!(parse_edge("right").unwrap(), Edge::Right));
        assert!(matches!(parse_edge("top").unwrap(), Edge::Top));
        assert!(matches!(parse_edge("bottom").unwrap(), Edge::Bottom));

        // Test case insensitivity
        assert!(matches!(parse_edge("LEFT").unwrap(), Edge::Left));
        assert!(matches!(parse_edge("Left").unwrap(), Edge::Left));

        // Test invalid values
        assert!(parse_edge("invalid").is_err());
    }

    #[test]
    fn test_parse_alignment() {
        use crate::types::Alignment;

        assert!(matches!(parse_alignment("left").unwrap(), Alignment::Left));
        assert!(matches!(
            parse_alignment("right").unwrap(),
            Alignment::Right
        ));
        assert!(matches!(parse_alignment("top").unwrap(), Alignment::Top));
        assert!(matches!(
            parse_alignment("bottom").unwrap(),
            Alignment::Bottom
        ));
        assert!(matches!(
            parse_alignment("center").unwrap(),
            Alignment::Center
        ));

        // Test invalid values
        assert!(parse_alignment("invalid").is_err());
    }

    #[test]
    fn test_parse_monitor_direction() {
        use crate::types::MonitorDirection;

        assert!(matches!(
            parse_monitor_direction("next").unwrap(),
            MonitorDirection::Next
        ));
        assert!(matches!(
            parse_monitor_direction("prev").unwrap(),
            MonitorDirection::Prev
        ));
        assert!(matches!(
            parse_monitor_direction("previous").unwrap(),
            MonitorDirection::Prev
        ));

        // Test numeric index
        if let MonitorDirection::Index(idx) = parse_monitor_direction("2").unwrap() {
            assert_eq!(idx, 2);
        } else {
            panic!("Expected Index direction");
        }

        // Test invalid values
        assert!(parse_monitor_direction("invalid").is_err());
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.log_level, "info");
        assert!(config.tray_icon);
        assert!(config.auto_reload);
        assert!(config.keyboard.remap.is_empty());
        assert!(config.keyboard.layers.is_empty());
    }

    #[test]
    fn test_config_from_str_minimal() {
        let config_str = r#"
[keyboard.remap]
CapsLock = "Backspace"
"#;

        let config = Config::from_str(config_str).unwrap();
        assert_eq!(config.log_level, "info"); // default value
        assert!(config.keyboard.remap.contains_key("CapsLock"));
    }

    #[test]
    fn test_config_full() {
        let config_str = r#"
log_level = "debug"
tray_icon = false
auto_reload = false

[keyboard.remap]
CapsLock = "Backspace"
Escape = "CapsLock"

[keyboard.layers.vim]
activation_key = "RightAlt"
mode = "Hold"

[keyboard.layers.vim.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"

[window.shortcuts]
"Ctrl+Alt+C" = "Center"
"Ctrl+Alt+Left" = "HalfScreen(Left)"

[launch]
F1 = "notepad.exe"
F2 = "calc.exe"

[network]
enabled = true
instance_id = 1
auth_key = "secret"

[macros]
test_macro = []
"#;

        let config = Config::from_str(config_str).unwrap();
        assert_eq!(config.log_level, "debug");
        assert!(!config.tray_icon);
        assert!(!config.auto_reload);
        assert_eq!(config.keyboard.remap.len(), 2);
        assert_eq!(config.keyboard.layers.len(), 1);
        assert_eq!(config.window.shortcuts.len(), 2);
        assert_eq!(config.launch.len(), 2);
        assert!(config.network.enabled);
        assert_eq!(config.network.instance_id, 1);
    }

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.instance_id, 0);
        assert!(config.auth_key.is_none());
    }

    #[test]
    fn test_network_config_get_bind_address() {
        let config = NetworkConfig {
            instance_id: 0,
            ..Default::default()
        };
        assert_eq!(config.get_bind_address(), "127.0.0.1:57427");

        let config = NetworkConfig {
            instance_id: 5,
            ..Default::default()
        };
        assert_eq!(config.get_bind_address(), "127.0.0.1:57432");
    }

    #[test]
    fn test_wildcard_match_function() {
        // Test public wildcard matching function
        assert!(wildcard_match("test.exe", "*.exe"));
        assert!(wildcard_match("file.txt", "*.txt"));
        assert!(wildcard_match("document.pdf", "*.pdf"));
        assert!(!wildcard_match("test.exe", "*.txt"));
    }

    #[test]
    fn test_wildcard_dp_basic_patterns() {
        assert!(wildcard_match("hello", "hello"));
        assert!(!wildcard_match("hello", "world"));

        assert!(wildcard_match("test.exe", "*.exe"));
        assert!(wildcard_match("file.txt", "*.txt"));
        assert!(wildcard_match("", "*"));
        assert!(wildcard_match("anything", "*"));
        assert!(wildcard_match("prefix-suffix", "*suffix"));
        assert!(wildcard_match("prefix-suffix", "prefix*"));

        assert!(wildcard_match("cat", "?at"));
        assert!(wildcard_match("bat", "?at"));
        assert!(!wildcard_match("at", "?at"));
        assert!(wildcard_match("abc", "???"));
        assert!(!wildcard_match("ab", "???"));

        assert!(wildcard_match("test123.txt", "test*.txt"));
        assert!(wildcard_match("file_1.txt", "file_?.txt"));
    }

    #[test]
    fn test_wildcard_dp_edge_cases() {
        assert!(wildcard_match("", ""));
        assert!(!wildcard_match("a", ""));
        assert!(wildcard_match("", "*"));
        assert!(!wildcard_match("", "?"));

        assert!(wildcard_match("test", "**test"));
        assert!(wildcard_match("test", "***"));
        assert!(wildcard_match("", "**"));

        assert!(wildcard_match("test", "****test"));

        assert!(wildcard_match("TEST.EXE", "*.exe"));
        assert!(wildcard_match("File.TXT", "*.txt"));
    }

    #[test]
    fn test_wildcard_dp_complex_patterns() {
        assert!(wildcard_match("a.b.c.d", "*.d"));
        assert!(wildcard_match("a.b.c.d", "a.*.c.*"));

        assert!(wildcard_match("test_2024.log", "test_????.log"));
        assert!(wildcard_match("image001.png", "image???.png"));

        assert!(wildcard_match("/path/to/file.txt", "/path/*/file.txt"));
        assert!(wildcard_match(
            "c:\\users\\test\\*\\*.txt",
            "c:\\users\\test\\*\\*.txt"
        ));
    }

    #[test]
    fn test_wildcard_dp_performance_safety() {
        let long_text = "a".repeat(1000);
        let long_pattern = "*".repeat(100);

        let result = wildcard_match(&long_text, &long_pattern);
        assert!(result);

        assert!(!wildcard_match(&long_text, ""));

        assert!(wildcard_match(&long_text, "*"));
    }

    #[test]
    fn test_parse_shortcut_trigger() {
        use crate::types::Trigger;

        let trigger = parse_shortcut_trigger("Ctrl+Alt+C").unwrap();
        if let Trigger::Key {
            scan_code: _,
            virtual_key: _,
            modifiers,
            ..
        } = trigger
        {
            assert!(modifiers.ctrl);
            assert!(modifiers.alt);
            assert!(!modifiers.shift);
            assert!(!modifiers.meta);
        } else {
            panic!("Expected Key trigger");
        }

        // Test with Win key
        let trigger = parse_shortcut_trigger("Ctrl+Win+Left").unwrap();
        if let Trigger::Key { modifiers, .. } = trigger {
            assert!(modifiers.ctrl);
            assert!(modifiers.meta);
        } else {
            panic!("Expected Key trigger");
        }
    }

    #[test]
    fn test_parse_shortcut_trigger_invalid() {
        // Empty shortcut
        assert!(parse_shortcut_trigger("").is_err());

        // Modifiers only
        assert!(parse_shortcut_trigger("Ctrl+Alt").is_err());
    }

    #[test]
    fn test_parse_launch_mapping() {
        let rule = parse_launch_mapping("F1", "notepad.exe").unwrap();

        // Verify trigger
        if let crate::types::Trigger::Key { virtual_key, .. } = &rule.trigger {
            assert_eq!(*virtual_key, Some(0x70)); // VK_F1
        } else {
            panic!("Expected Key trigger");
        }

        // Verify action
        if let crate::types::Action::Launch(cmd) = &rule.action {
            assert_eq!(cmd.program, "notepad.exe");
        } else {
            panic!("Expected Launch action");
        }
    }

    #[test]
    fn test_parse_launch_mapping_with_modifiers() {
        // Test launch mapping with modifier keys (e.g., "Ctrl+Alt+Meta+T" = "wt.exe")
        let rule = parse_launch_mapping("Ctrl+Alt+Meta+T", "wt.exe").unwrap();

        // Verify trigger has correct modifiers
        if let crate::types::Trigger::Key {
            virtual_key,
            modifiers,
            ..
        } = &rule.trigger
        {
            assert_eq!(*virtual_key, Some(0x54)); // VK_T
            assert!(modifiers.ctrl, "Should have Ctrl modifier");
            assert!(modifiers.alt, "Should have Alt modifier");
            assert!(modifiers.meta, "Should have Meta/Win modifier");
            assert!(!modifiers.shift, "Should not have Shift modifier");
        } else {
            panic!("Expected Key trigger");
        }

        // Verify action
        if let crate::types::Action::Launch(cmd) = &rule.action {
            assert_eq!(cmd.program, "wt.exe");
            assert!(cmd.args.is_empty());
        } else {
            panic!("Expected Launch action");
        }
    }

    #[test]
    fn test_parse_launch_mapping_with_args() {
        // Test launch mapping with command arguments
        let rule =
            parse_launch_mapping("Ctrl+Alt+Meta+N", "notepad.exe C:\\test.txt").unwrap();

        // Verify trigger
        if let crate::types::Trigger::Key {
            virtual_key,
            modifiers,
            ..
        } = &rule.trigger
        {
            assert_eq!(*virtual_key, Some(0x4E)); // VK_N
            assert!(modifiers.ctrl);
            assert!(modifiers.alt);
            assert!(modifiers.meta);
        } else {
            panic!("Expected Key trigger");
        }

        // Verify action with arguments
        if let crate::types::Action::Launch(cmd) = &rule.action {
            assert_eq!(cmd.program, "notepad.exe");
            assert_eq!(cmd.args, vec!["C:\\test.txt"]);
        } else {
            panic!("Expected Launch action");
        }
    }

    #[test]
    fn test_mouse_config_default() {
        let config = MouseConfig::default();
        assert!(config.button_remap.is_empty());
        assert_eq!(config.wheel.speed, 3);
        assert!(!config.wheel.invert);
        assert!(!config.wheel.acceleration);
        assert!((config.wheel.acceleration_multiplier - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_wheel_config_default() {
        let config = WheelConfig::default();
        assert_eq!(config.speed, 3);
        assert!(!config.invert);
        assert!(!config.acceleration);
    }

    #[test]
    fn test_config_get_all_rules() {
        let config_str = r#"
[keyboard.remap]
CapsLock = "Backspace"

[window.shortcuts]
"Ctrl+Alt+C" = "Center"
"#;

        let config = Config::from_str(config_str).unwrap();
        let rules = config.get_all_rules();
        assert!(!rules.is_empty());
    }

    #[test]
    fn test_config_with_macros() {
        let config_str = r#"
[macros]
test_macro = []

[macro_bindings]
F5 = "test_macro"
"#;

        let config = Config::from_str(config_str).unwrap();
        assert!(config.macros.contains_key("test_macro"));
        assert_eq!(
            config.macro_bindings.get("F5"),
            Some(&"test_macro".to_string())
        );
    }

    #[test]
    fn test_validate_empty_auth_key_rejected() {
        let mut config = Config::default();
        config.network.auth_key = Some(String::new());
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("empty string") || err.contains("auth_key"),
            "Expected empty auth_key error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_instance_id_out_of_range() {
        let mut config = Config::default();
        config.network.instance_id = 256;
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("instance_id") || err.contains("0-255"),
            "Expected instance_id range error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_invalid_remap_key() {
        let config_str = r#"
[keyboard.remap]
"NonExistentKey123" = "A"
"#;
        let config: Config = toml::from_str(config_str).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("keyboard.remap") || err.contains("NonExistentKey123"),
            "Expected invalid key error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_invalid_shortcut_key() {
        let config_str = r#"
[window.shortcuts]
"BogusKey" = "Center"
"#;
        let config: Config = toml::from_str(config_str).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("window.shortcuts") || err.contains("BogusKey"),
            "Expected invalid shortcut error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_invalid_shortcut_action() {
        let config_str = r#"
[window.shortcuts]
"Ctrl+Alt+C" = "NonExistentAction"
"#;
        let config: Config = toml::from_str(config_str).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("window.shortcuts") || err.contains("NonExistentAction"),
            "Expected invalid action error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_invalid_layer_activation_key() {
        let config_str = r#"
[keyboard.layers.bad]
activation_key = "NotARealKey999"
mode = "Hold"

[keyboard.layers.bad.mappings]
H = "Left"
"#;
        let config: Config = toml::from_str(config_str).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("activation_key") || err.contains("NotARealKey999"),
            "Expected invalid activation_key error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_invalid_layer_mapping_key() {
        let config_str = r#"
[keyboard.layers.bad]
activation_key = "CapsLock"
mode = "Hold"

[keyboard.layers.bad.mappings]
"InvalidKey999" = "Left"
"#;
        let config: Config = toml::from_str(config_str).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("InvalidKey999"),
            "Expected invalid layer mapping key error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_invalid_layer_mapping_target() {
        let config_str = r#"
[keyboard.layers.bad]
activation_key = "CapsLock"
mode = "Hold"

[keyboard.layers.bad.mappings]
H = "NotAKeyOrAction999"
"#;
        let config: Config = toml::from_str(config_str).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("NotAKeyOrAction999"),
            "Expected invalid layer target error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_invalid_launch_trigger() {
        let config_str = r#"
[launch]
"InvalidTrigger999" = "notepad.exe"
"#;
        let config: Config = toml::from_str(config_str).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("launch") || err.contains("InvalidTrigger999"),
            "Expected invalid launch trigger error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_valid_config_passes() {
        let config_str = r#"
[keyboard.remap]
CapsLock = "Backspace"

[keyboard.layers.nav]
activation_key = "RightAlt"
mode = "Hold"

[keyboard.layers.nav.mappings]
H = "Left"

[window.shortcuts]
"Ctrl+Alt+C" = "Center"

[launch]
F1 = "notepad.exe"

[network]
instance_id = 0
auth_key = "valid_key"
"#;
        let config: Config = toml::from_str(config_str).unwrap();
        assert!(config.validate().is_ok());
    }
}
