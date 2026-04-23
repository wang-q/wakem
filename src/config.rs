use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use tracing::debug;

use keyboard_codes::{Key, KeyCodeMapper, Platform};

use crate::constants::{
    DEFAULT_ACCELERATION_MULTIPLIER, DEFAULT_WHEEL_SPEED, DEFAULT_WHEEL_STEP,
    WILDCARD_MAX_INPUT_SIZE,
};
use crate::types::{ContextCondition, MacroStep, MappingRule};

#[cfg(target_os = "windows")]
use crate::platform::windows::Launcher;

#[cfg(target_os = "macos")]
use crate::platform::macos::Launcher;

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

        // 2. Validate network port range (u16 max is 65535, only need to check minimum)
        let port = crate::ipc::get_instance_port(self.network.instance_id);
        if port < 1024 {
            anyhow::bail!("Invalid port {}: must be in range 1024-65535", port);
        }

        // 3. Validate instance ID range
        if self.network.instance_id > 255 {
            anyhow::bail!(
                "Invalid instance_id {}: must be in range 0-255",
                self.network.instance_id
            );
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

        Ok(())
    }

    /// Save configuration to file
    #[allow(dead_code)]
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get all mapping rules
    pub fn get_all_rules(&self) -> Vec<MappingRule> {
        let mut rules = Vec::new();
        rules.extend(
            self.keyboard
                .remap
                .iter()
                .filter_map(|(k, v)| parse_key_mapping(k, v).ok()),
        );
        rules.extend(
            self.keyboard
                .layers
                .iter()
                .filter_map(|(name, layer)| self.parse_layer_mappings(name, layer).ok())
                .flatten(),
        );
        rules.extend(
            self.window
                .shortcuts
                .iter()
                .filter_map(|(k, v)| parse_window_shortcut(k, v).ok()),
        );
        // Add launch item mappings
        rules.extend(
            self.launch
                .iter()
                .filter_map(|(k, v)| parse_launch_mapping(k, v).ok()),
        );
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
    /// Get instance communication port
    #[allow(dead_code)]
    pub fn get_port(&self) -> u16 {
        crate::ipc::get_instance_port(self.instance_id)
    }

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
        let mut rng = rand::thread_rng();
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

impl WindowPreset {
    /// Check if preset matches specified window info
    pub fn matches(
        &self,
        process_name: &str,
        executable_path: Option<&str>,
        title: &str,
    ) -> bool {
        // Check process name match
        if let Some(ref pattern) = self.process_name {
            if !Self::wildcard_match(process_name, pattern) {
                return false;
            }
        }

        // Check executable path match
        if let Some(ref pattern) = self.executable_path {
            let path = executable_path.unwrap_or("");
            if !Self::wildcard_match(path, pattern) {
                return false;
            }
        }

        // Check window title match
        if let Some(ref pattern) = self.title_pattern {
            if !Self::wildcard_match(title, pattern) {
                return false;
            }
        }

        // At least one matching condition is required
        self.process_name.is_some()
            || self.executable_path.is_some()
            || self.title_pattern.is_some()
    }

    /// Simple wildcard matching (* matches any characters, ? matches single character)
    fn wildcard_match(text: &str, pattern: &str) -> bool {
        wildcard_match(text, pattern)
    }
}

/// Public wildcard matching function (supports * and ?)
/// Unified implementation to avoid code duplication
///
/// Performance optimizations:
/// - Fast path for exact matches and simple patterns
/// - Uses dynamic programming (DP) for complex patterns
/// - Time complexity: O(m*n) worst case, O(1) best case
/// - Prevents stack overflow and exponential time complexity
pub fn wildcard_match(text: &str, pattern: &str) -> bool {
    // Fast path 1: Exact match (case-insensitive)
    if text.eq_ignore_ascii_case(pattern) {
        return true;
    }

    // Fast path 2: Pattern is "*" (matches everything)
    if pattern == "*" {
        return true;
    }

    // Fast path 3: No wildcards - simple string comparison
    if !pattern.contains('*') && !pattern.contains('?') {
        return text.eq_ignore_ascii_case(pattern);
    }

    // Fast path 4: Pattern starts/ends with * (suffix/prefix match)
    if pattern.starts_with('*') && !pattern[1..].contains('*') && !pattern.contains('?')
    {
        let suffix = &pattern[1..];
        return text.to_lowercase().ends_with(&suffix.to_lowercase());
    }
    if pattern.ends_with('*')
        && !pattern[..pattern.len() - 1].contains('*')
        && !pattern.contains('?')
    {
        let prefix = &pattern[..pattern.len() - 1];
        return text.to_lowercase().starts_with(&prefix.to_lowercase());
    }

    // Full DP implementation for complex patterns
    let text_lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();
    wildcard_match_dp(&text_lower, &pattern_lower)
}

/// Dynamic programming implementation of wildcard matching
///
/// Algorithm description:
/// - dp[i][j] indicates whether text[0..i] matches pattern[0..j]
/// - State transitions:
///   - If pattern[j-1] == '*', can match 0 or more characters
///   - If pattern[j-1] == '?' or characters are equal, match current character
fn wildcard_match_dp(text: &str, pattern: &str) -> bool {
    let text_chars: Vec<char> = text.to_lowercase().chars().collect();
    let pattern_chars: Vec<char> = pattern.to_lowercase().chars().collect();

    let m = text_chars.len();
    let n = pattern_chars.len();

    // Boundary case handling
    if n == 0 {
        return m == 0;
    }

    // Prevent large inputs from causing memory issues
    if m > WILDCARD_MAX_INPUT_SIZE || n > WILDCARD_MAX_INPUT_SIZE {
        return false;
    }

    // Create DP table (m+1) x (n+1)
    let mut dp = vec![vec![false; n + 1]; m + 1];

    // Empty string matches empty pattern
    dp[0][0] = true;

    // Handle '*' at the beginning of pattern (can match empty string)
    for j in 1..=n {
        if pattern_chars[j - 1] == '*' {
            dp[0][j] = dp[0][j - 1];
        } else {
            break; // Stop when encountering non-'*' character
        }
    }

    // Fill DP table
    for i in 1..=m {
        for j in 1..=n {
            match pattern_chars[j - 1] {
                '*' => {
                    // '*' can match:
                    // 1. 0 characters (dp[i][j-1])
                    // 2. 1 or more characters (dp[i-1][j])
                    dp[i][j] = dp[i][j - 1] || dp[i - 1][j];
                }
                '?' => {
                    // '?' matches any single character
                    dp[i][j] = dp[i - 1][j - 1];
                }
                _ => {
                    // Regular characters must match exactly (already converted to lowercase)
                    dp[i][j] =
                        dp[i - 1][j - 1] && (text_chars[i - 1] == pattern_chars[j - 1]);
                }
            }
        }
    }

    dp[m][n]
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
    /// Volume control configuration
    #[serde(default)]
    pub volume_control: Option<WheelModifierConfig>,
    /// Brightness control configuration
    #[serde(default)]
    pub brightness_control: Option<WheelModifierConfig>,
}

impl Default for WheelConfig {
    fn default() -> Self {
        Self {
            speed: default_wheel_speed(),
            invert: false,
            acceleration: false,
            acceleration_multiplier: default_acceleration_multiplier(),
            horizontal_scroll: None,
            volume_control: None,
            brightness_control: None,
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

    // Press modifier keys (in specific order: Ctrl -> Alt -> Win -> Shift)
    if modifiers.ctrl {
        press_actions.push(Action::key(KeyAction::press(0x1D, 0x11))); // Ctrl
        release_actions.insert(0, Action::key(KeyAction::release(0x1D, 0x11))); // Ctrl (reverse)
    }
    if modifiers.alt {
        press_actions.push(Action::key(KeyAction::press(0x38, 0x12))); // Alt
        release_actions.insert(0, Action::key(KeyAction::release(0x38, 0x12))); // Alt (reverse)
    }
    if modifiers.meta {
        press_actions.push(Action::key(KeyAction::press(0x5B, 0x5B))); // Win (Left)
        release_actions.insert(0, Action::key(KeyAction::release(0x5B, 0x5B))); // Win (reverse)
    }
    if modifiers.shift {
        press_actions.push(Action::key(KeyAction::press(0x2A, 0x10))); // Shift
        release_actions.insert(0, Action::key(KeyAction::release(0x2A, 0x10))); // Shift (reverse)
    }

    // Combine press and release into a sequence that will be split by the mapper
    // The mapper will execute press_actions on key down and release_actions on key up
    let mut all_actions = press_actions;
    // Add a small delay after pressing modifiers to ensure system recognizes them
    all_actions.push(Action::Delay { milliseconds: 10 });
    all_actions.push(Action::None); // Marker to split press and release actions
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
                let edge = parse_edge(param_list.first().unwrap_or(&""))?;
                Ok(WindowAction::MoveToEdge(edge))
            }
            "HalfScreen" => {
                let edge = parse_edge(param_list.first().unwrap_or(&""))?;
                Ok(WindowAction::HalfScreen(edge))
            }
            "LoopWidth" => {
                let align = parse_alignment(param_list.first().unwrap_or(&""))?;
                Ok(WindowAction::LoopWidth(align))
            }
            "LoopHeight" => {
                let align = parse_alignment(param_list.first().unwrap_or(&""))?;
                Ok(WindowAction::LoopHeight(align))
            }
            "FixedRatio" => {
                let ratio = param_list.first().unwrap_or(&"1.333").parse::<f32>()?;
                let scale_index = param_list.get(1).unwrap_or(&"0").parse::<usize>()?;
                Ok(WindowAction::FixedRatio { ratio, scale_index })
            }
            "NativeRatio" => {
                let scale_index = param_list.first().unwrap_or(&"0").parse::<usize>()?;
                Ok(WindowAction::NativeRatio { scale_index })
            }
            "MoveToMonitor" => {
                let direction =
                    parse_monitor_direction(param_list.first().unwrap_or(&""))?;
                Ok(WindowAction::MoveToMonitor(direction))
            }
            "Move" => {
                let x = param_list.first().unwrap_or(&"0").parse::<i32>()?;
                let y = param_list.get(1).unwrap_or(&"0").parse::<i32>()?;
                Ok(WindowAction::Move { x, y })
            }
            "Resize" => {
                let width = param_list.first().unwrap_or(&"800").parse::<i32>()?;
                let height = param_list.get(1).unwrap_or(&"600").parse::<i32>()?;
                Ok(WindowAction::Resize { width, height })
            }
            "ShowNotification" => {
                let title = param_list.first().unwrap_or(&"wakem").to_string();
                let message = param_list.get(1).unwrap_or(&"").to_string();
                Ok(WindowAction::ShowNotification { title, message })
            }
            "SavePreset" => {
                let name = param_list.first().unwrap_or(&"default").to_string();
                Ok(WindowAction::SavePreset { name })
            }
            "LoadPreset" => {
                let name = param_list.first().unwrap_or(&"default").to_string();
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
/// Uses match expression for O(1) lookup without heap allocation
pub fn parse_key(name: &str) -> anyhow::Result<(u16, u16)> {
    let name_lower = name.to_lowercase();

    // Try keyboard-codes first (supports standard key names)
    if let Ok(key) = name_lower.parse::<Key>() {
        let win_code = key.to_code(Platform::Windows) as u16;
        if win_code != 0 || !name_lower.is_empty() {
            return Ok((win_code, win_code));
        }
    }

    // Fallback to legacy hardcoded mappings for backward compatibility
    match name_lower.as_str() {
        // Special keys
        "capslock" | "caps" => Ok((0x3A, 0x14)),
        "backspace" => Ok((0x0E, 0x08)),
        "enter" | "return" => Ok((0x1C, 0x0D)),
        "escape" | "esc" => Ok((0x01, 0x1B)),
        "space" => Ok((0x39, 0x20)),
        "tab" => Ok((0x0F, 0x09)),
        "grave" | "backtick" => Ok((0x29, 0xC0)),

        // Arrow keys
        "left" => Ok((0x4B, 0x25)),
        "up" => Ok((0x48, 0x26)),
        "right" => Ok((0x4D, 0x27)),
        "down" => Ok((0x50, 0x28)),

        // Editing keys
        "home" => Ok((0x47, 0x24)),
        "end" => Ok((0x4F, 0x23)),
        "pageup" => Ok((0x49, 0x21)),
        "pagedown" => Ok((0x51, 0x22)),
        "delete" | "del" | "forwarddelete" | "forwarddel" => Ok((0x53, 0x2E)),
        "insert" | "ins" => Ok((0x52, 0x2D)),

        // Modifier keys
        "lshift" => Ok((0x2A, 0xA0)),
        "rshift" => Ok((0x36, 0xA1)),
        "lctrl" | "lcontrol" => Ok((0x1D, 0xA2)),
        "rctrl" | "rcontrol" => Ok((0xE01D, 0xA3)),
        "lalt" => Ok((0x38, 0xA4)),
        "ralt" => Ok((0xE038, 0xA5)),
        "lwin" | "lmeta" => Ok((0xE05B, 0x5B)),
        "rwin" | "rmeta" => Ok((0xE05C, 0x5C)),

        // Letter keys a-z
        "a" => Ok((0x1E, 0x41)),
        "b" => Ok((0x30, 0x42)),
        "c" => Ok((0x2E, 0x43)),
        "d" => Ok((0x20, 0x44)),
        "e" => Ok((0x12, 0x45)),
        "f" => Ok((0x21, 0x46)),
        "g" => Ok((0x22, 0x47)),
        "h" => Ok((0x23, 0x48)),
        "i" => Ok((0x17, 0x49)),
        "j" => Ok((0x24, 0x4A)),
        "k" => Ok((0x25, 0x4B)),
        "l" => Ok((0x26, 0x4C)),
        "m" => Ok((0x32, 0x4D)),
        "n" => Ok((0x31, 0x4E)),
        "o" => Ok((0x18, 0x4F)),
        "p" => Ok((0x19, 0x50)),
        "q" => Ok((0x10, 0x51)),
        "r" => Ok((0x13, 0x52)),
        "s" => Ok((0x1F, 0x53)),
        "t" => Ok((0x14, 0x54)),
        "u" => Ok((0x16, 0x55)),
        "v" => Ok((0x2F, 0x56)),
        "w" => Ok((0x11, 0x57)),
        "x" => Ok((0x2D, 0x58)),
        "y" => Ok((0x15, 0x59)),
        "z" => Ok((0x2C, 0x5A)),

        // Number keys 0-9
        "0" => Ok((0x0B, 0x30)),
        "1" => Ok((0x02, 0x31)),
        "2" => Ok((0x03, 0x32)),
        "3" => Ok((0x04, 0x33)),
        "4" => Ok((0x05, 0x34)),
        "5" => Ok((0x06, 0x35)),
        "6" => Ok((0x07, 0x36)),
        "7" => Ok((0x08, 0x37)),
        "8" => Ok((0x09, 0x38)),
        "9" => Ok((0x0A, 0x39)),

        // Function keys F1-F12
        "f1" => Ok((0x3B, 0x70)),
        "f2" => Ok((0x3C, 0x71)),
        "f3" => Ok((0x3D, 0x72)),
        "f4" => Ok((0x3E, 0x73)),
        "f5" => Ok((0x3F, 0x74)),
        "f6" => Ok((0x40, 0x75)),
        "f7" => Ok((0x41, 0x76)),
        "f8" => Ok((0x42, 0x77)),
        "f9" => Ok((0x43, 0x78)),
        "f10" => Ok((0x44, 0x79)),
        "f11" => Ok((0x57, 0x7A)),
        "f12" => Ok((0x58, 0x7B)),

        // Punctuation keys (US layout)
        "comma" | "," => Ok((0x33, 0xBC)), // VK_OEM_COMMA
        "period" | "." => Ok((0x34, 0xBE)), // VK_OEM_PERIOD
        "semicolon" | ";" => Ok((0x27, 0xBA)), // VK_OEM_1
        "quote" | "'" | "apostrophe" => Ok((0x28, 0xDE)), // VK_OEM_7
        "bracketleft" | "[" => Ok((0x1A, 0xDB)), // VK_OEM_4
        "bracketright" | "]" => Ok((0x1B, 0xDD)), // VK_OEM_6
        "backslash" | "\\" => Ok((0x2B, 0xDC)), // VK_OEM_5
        "minus" | "-" => Ok((0x0C, 0xBD)), // VK_OEM_MINUS
        "equal" | "=" => Ok((0x0D, 0xBB)), // VK_OEM_PLUS

        // Numpad keys
        "numpad0" | "num0" => Ok((0x52, 0x60)),
        "numpad1" | "num1" => Ok((0x4F, 0x61)),
        "numpad2" | "num2" => Ok((0x50, 0x62)),
        "numpad3" | "num3" => Ok((0x51, 0x63)),
        "numpad4" | "num4" => Ok((0x4B, 0x64)),
        "numpad5" | "num5" => Ok((0x4C, 0x65)),
        "numpad6" | "num6" => Ok((0x4D, 0x66)),
        "numpad7" | "num7" => Ok((0x47, 0x67)),
        "numpad8" | "num8" => Ok((0x48, 0x68)),
        "numpad9" | "num9" => Ok((0x49, 0x69)),
        "numpaddot" | "numdot" | "numpaddecimal" => Ok((0x53, 0x6E)),
        "numpadenter" | "numenter" => Ok((0x1C, 0x0C)),
        "numpadadd" | "numplus" => Ok((0x4E, 0x6B)),
        "numpadsub" | "numminus" => Ok((0x4A, 0x6D)),
        "numpadmul" | "nummul" | "numpadmultiply" => Ok((0x37, 0x6A)),
        "numpaddiv" | "numslash" | "numpaddivide" => Ok((0x35, 0x6F)),

        _ => Err(anyhow::anyhow!("Unknown key name: {}", name)),
    }
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
        // Check cache first
        if let Ok(mut cache) = self.cache.lock() {
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
        } else {
            // Fallback to direct resolution when lock fails
            Self::resolve_config_path_internal(instance_id)
        }
    }

    /// Invalidate cache for specified instance
    #[allow(dead_code)]
    fn invalidate(&self, instance_id: u32) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.remove(&instance_id);
            debug!("Invalidated config path cache for instance {}", instance_id);
        }
    }

    /// Clear all cache
    #[allow(dead_code)]
    fn clear(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
            debug!("Cleared all config path cache");
        }
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

/// Invalidate config file path cache
///
/// Call this function after config file is moved, renamed, or deleted
#[allow(dead_code)]
pub fn invalidate_config_path_cache(instance_id: u32) {
    CONFIG_PATH_CACHE.invalidate(instance_id);
}

/// Clear all config file path cache
#[allow(dead_code)]
pub fn clear_config_path_cache() {
    CONFIG_PATH_CACHE.clear();
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
        // Test CapsLock -> Ctrl+Alt+Win mapping
        let rule = parse_key_mapping("CapsLock", "Ctrl+Alt+Win").unwrap();

        // Verify trigger is CapsLock
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

        // Verify action is Sequence (contains modifier key press/release)
        if let crate::types::Action::Sequence(actions) = &rule.action {
            // Should have 8 actions: Ctrl press, Alt press, Win press, Delay, None marker, Win release, Alt release, Ctrl release
            assert_eq!(actions.len(), 8);

            // Verify first action is Ctrl press
            if let crate::types::Action::Key(crate::types::KeyAction::Press {
                virtual_key,
                ..
            }) = &actions[0]
            {
                assert_eq!(*virtual_key, 0x11); // VK_CONTROL
            } else {
                panic!("Expected Ctrl Press as first action, got {:?}", actions[0]);
            }

            // Verify second action is Alt press
            if let crate::types::Action::Key(crate::types::KeyAction::Press {
                virtual_key,
                ..
            }) = &actions[1]
            {
                assert_eq!(*virtual_key, 0x12); // VK_MENU (Alt)
            } else {
                panic!("Expected Alt Press as second action, got {:?}", actions[1]);
            }

            // Verify third action is Win press
            if let crate::types::Action::Key(crate::types::KeyAction::Press {
                virtual_key,
                ..
            }) = &actions[2]
            {
                assert_eq!(*virtual_key, 0x5B); // VK_LWIN
            } else {
                panic!("Expected Win Press as third action, got {:?}", actions[2]);
            }

            // Verify fourth action is Delay (10ms)
            if let crate::types::Action::Delay { milliseconds } = &actions[3] {
                assert_eq!(*milliseconds, 10);
            } else {
                panic!("Expected Delay as fourth action, got {:?}", actions[3]);
            }

            // Verify fifth action is None (marker to split press and release)
            assert!(
                matches!(actions[4], crate::types::Action::None),
                "Expected None marker as fifth action, got {:?}",
                actions[4]
            );

            // Verify sixth, seventh, eighth actions are release
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
    fn test_window_preset_matches() {
        let preset = WindowPreset {
            name: "test".to_string(),
            process_name: Some("chrome.exe".to_string()),
            executable_path: None,
            title_pattern: None,
            x: 100,
            y: 100,
            width: 800,
            height: 600,
        };

        assert!(preset.matches("chrome.exe", None, "Google Chrome"));
        assert!(!preset.matches("firefox.exe", None, "Firefox"));
    }

    #[test]
    fn test_window_preset_wildcard_match() {
        // Test wildcard matching
        assert!(WindowPreset::wildcard_match("chrome.exe", "*.exe"));
        assert!(WindowPreset::wildcard_match("test.txt", "*.txt"));
        assert!(WindowPreset::wildcard_match("abc", "a*c"));
        assert!(WindowPreset::wildcard_match("abc", "a?c"));
        assert!(!WindowPreset::wildcard_match("abc", "a?d"));
        assert!(WindowPreset::wildcard_match("ABC", "abc")); // case insensitive
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
        // basic matching
        assert!(wildcard_match_dp("hello", "hello"));
        assert!(!wildcard_match_dp("hello", "world"));

        // * wildcard (matches any character sequence)
        assert!(wildcard_match_dp("test.exe", "*.exe"));
        assert!(wildcard_match_dp("file.txt", "*.txt"));
        assert!(wildcard_match_dp("", "*"));
        assert!(wildcard_match_dp("anything", "*"));
        assert!(wildcard_match_dp("prefix-suffix", "*suffix"));
        assert!(wildcard_match_dp("prefix-suffix", "prefix*"));

        // ? wildcard (matches single character)
        assert!(wildcard_match_dp("cat", "?at"));
        assert!(wildcard_match_dp("bat", "?at"));
        assert!(!wildcard_match_dp("at", "?at")); // ? requires one character
        assert!(wildcard_match_dp("abc", "???"));
        assert!(!wildcard_match_dp("ab", "???"));

        // Mixed usage
        assert!(wildcard_match_dp("test123.txt", "test*.txt"));
        assert!(wildcard_match_dp("file_1.txt", "file_?.txt"));
    }

    #[test]
    fn test_wildcard_dp_edge_cases() {
        // empty string and empty pattern
        assert!(wildcard_match_dp("", ""));
        assert!(!wildcard_match_dp("a", ""));
        assert!(wildcard_match_dp("", "*"));
        assert!(!wildcard_match_dp("", "?")); // ? requires at least one character

        // consecutive *
        assert!(wildcard_match_dp("test", "**test"));
        assert!(wildcard_match_dp("test", "***"));
        assert!(wildcard_match_dp("", "**"));

        // multiple leading *
        assert!(wildcard_match_dp("test", "****test"));

        // case insensitive (converted to lowercase)
        assert!(wildcard_match_dp("TEST.EXE", "*.exe"));
        assert!(wildcard_match_dp("File.TXT", "*.txt"));
    }

    #[test]
    fn test_wildcard_dp_complex_patterns() {
        // multiple *
        assert!(wildcard_match_dp("a.b.c.d", "*.d"));
        assert!(wildcard_match_dp("a.b.c.d", "a.*.c.*"));

        // complex mixed patterns
        assert!(wildcard_match_dp("test_2024.log", "test_????.log"));
        assert!(wildcard_match_dp("image001.png", "image???.png"));

        // path-style matching
        assert!(wildcard_match_dp("/path/to/file.txt", "/path/*/file.txt"));
        assert!(wildcard_match_dp(
            "C:\\Users\\test\\*\\*.txt",
            "C:\\Users\\test\\*\\*.txt"
        ));
    }

    #[test]
    fn test_wildcard_dp_performance_safety() {
        // Test should not crash or stack overflow on long input
        let long_text = "a".repeat(1000);
        let long_pattern = "*".repeat(100);

        // Should handle normally without stack overflow
        let result = wildcard_match_dp(&long_text, &long_pattern);
        assert!(result); // * matches anything

        // empty pattern and long text
        assert!(!wildcard_match_dp(&long_text, ""));

        // long text and simple pattern
        assert!(wildcard_match_dp(&long_text, "*"));
    }

    #[test]
    fn test_parse_shortcut_trigger() {
        use crate::types::Trigger;

        let trigger = parse_shortcut_trigger("Ctrl+Alt+C").unwrap();
        if let Trigger::Key {
            scan_code,
            virtual_key,
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
}

/// Parse launch item mapping
/// Supported formats:
/// - Simple command: "notepad.exe"
/// - Command with arguments: "notepad.exe C:\\Users\\test.txt"
fn parse_launch_mapping(trigger: &str, command: &str) -> anyhow::Result<MappingRule> {
    use crate::types::{Action, Trigger};

    // Parse trigger key
    let (scan_code, virtual_key) = parse_key(trigger)?;
    let trigger_obj = Trigger::key(scan_code, virtual_key);

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
