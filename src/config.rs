use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use tracing::debug;

use crate::platform::windows::Launcher;
use crate::types::{ContextCondition, MacroStep, MappingRule};

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
/// - Uses dynamic programming (DP) instead of recursive implementation
/// - Time complexity: O(m*n), space complexity: O(m*n)
/// - Prevents stack overflow and exponential time complexity
pub fn wildcard_match(text: &str, pattern: &str) -> bool {
    let text = text.to_lowercase();
    let pattern = pattern.to_lowercase();

    wildcard_match_dp(&text, &pattern)
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
    const MAX_SIZE: usize = 1024;
    if m > MAX_SIZE || n > MAX_SIZE {
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
    3
}

fn default_acceleration_multiplier() -> f32 {
    2.0
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
    1
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
    if to.contains('+') && !to.contains("->") {
        // Parse modifier key combination (e.g., "Ctrl+Alt+Win")
        let modifiers = parse_modifier_combo(to)?;
        let trigger = Trigger::key(from_key.0, from_key.1);
        // Create action sequence for pressing/releasing modifier keys
        let action = create_modifier_press_release_action(&modifiers);
        return Ok(MappingRule::new(trigger, action));
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

/// Create action sequence for pressing and releasing modifier keys
/// When CapsLock is pressed, send Ctrl+Alt+Win press, send release on release
fn create_modifier_press_release_action(
    modifiers: &crate::types::ModifierState,
) -> crate::types::Action {
    use crate::types::{Action, KeyAction};

    let mut actions = Vec::new();

    // Press modifier keys (in specific order: Ctrl -> Alt -> Win -> Shift)
    if modifiers.ctrl {
        actions.push(Action::key(KeyAction::press(0x1D, 0x11))); // Ctrl
    }
    if modifiers.alt {
        actions.push(Action::key(KeyAction::press(0x38, 0x12))); // Alt
    }
    if modifiers.meta {
        actions.push(Action::key(KeyAction::press(0x5B, 0x5B))); // Win (Left)
    }
    if modifiers.shift {
        actions.push(Action::key(KeyAction::press(0x2A, 0x10))); // Shift
    }

    // Release modifier keys immediately (reverse order)
    if modifiers.shift {
        actions.push(Action::key(KeyAction::release(0x2A, 0x10))); // Shift
    }
    if modifiers.meta {
        actions.push(Action::key(KeyAction::release(0x5B, 0x5B))); // Win
    }
    if modifiers.alt {
        actions.push(Action::key(KeyAction::release(0x38, 0x12))); // Alt
    }
    if modifiers.ctrl {
        actions.push(Action::key(KeyAction::release(0x1D, 0x11))); // Ctrl
    }

    Action::Sequence(actions)
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
            "SetOpacity" => {
                let opacity = param_list.first().unwrap_or(&"255").parse::<u8>()?;
                Ok(WindowAction::SetOpacity { opacity })
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
/// Uses static HashMap for data-driven key name mapping, easy to maintain and extend
pub fn parse_key(name: &str) -> anyhow::Result<(u16, u16)> {
    use std::collections::HashMap;

    use once_cell::sync::Lazy;

    static KEY_MAP: Lazy<HashMap<&'static str, (u16, u16)>> = Lazy::new(|| {
        let mut map = HashMap::new();

        // Special keys
        map.insert("capslock", (0x3A, 0x14));
        map.insert("caps", (0x3A, 0x14));
        map.insert("backspace", (0x0E, 0x08));
        map.insert("enter", (0x1C, 0x0D));
        map.insert("return", (0x1C, 0x0D));
        map.insert("escape", (0x01, 0x1B));
        map.insert("esc", (0x01, 0x1B));
        map.insert("space", (0x39, 0x20));
        map.insert("tab", (0x0F, 0x09));

        // Arrow keys
        map.insert("left", (0x4B, 0x25));
        map.insert("up", (0x48, 0x26));
        map.insert("right", (0x4D, 0x27));
        map.insert("down", (0x50, 0x28));

        // Editing keys
        map.insert("home", (0x47, 0x24));
        map.insert("end", (0x4F, 0x23));
        map.insert("pageup", (0x49, 0x21));
        map.insert("pagedown", (0x51, 0x22));
        map.insert("delete", (0x53, 0x2E));
        map.insert("del", (0x53, 0x2E));
        map.insert("forwarddelete", (0x53, 0x2E));
        map.insert("forwarddel", (0x53, 0x2E));
        map.insert("insert", (0x52, 0x2D));
        map.insert("ins", (0x52, 0x2D));

        // Modifier keys
        map.insert("lshift", (0x2A, 0xA0));
        map.insert("rshift", (0x36, 0xA1));
        map.insert("lctrl", (0x1D, 0xA2));
        map.insert("lcontrol", (0x1D, 0xA2));
        map.insert("rctrl", (0xE01D, 0xA3));
        map.insert("rcontrol", (0xE01D, 0xA3));
        map.insert("lalt", (0x38, 0xA4));
        map.insert("ralt", (0xE038, 0xA5));
        map.insert("lwin", (0xE05B, 0x5B));
        map.insert("lmeta", (0xE05B, 0x5B));
        map.insert("rwin", (0xE05C, 0x5C));
        map.insert("rmeta", (0xE05C, 0x5C));

        // Letter keys a-z
        let letter_keys = [
            ('a', 0x1E, 0x41),
            ('b', 0x30, 0x42),
            ('c', 0x2E, 0x43),
            ('d', 0x20, 0x44),
            ('e', 0x12, 0x45),
            ('f', 0x21, 0x46),
            ('g', 0x22, 0x47),
            ('h', 0x23, 0x48),
            ('i', 0x17, 0x49),
            ('j', 0x24, 0x4A),
            ('k', 0x25, 0x4B),
            ('l', 0x26, 0x4C),
            ('m', 0x32, 0x4D),
            ('n', 0x31, 0x4E),
            ('o', 0x18, 0x4F),
            ('p', 0x19, 0x50),
            ('q', 0x10, 0x51),
            ('r', 0x13, 0x52),
            ('s', 0x1F, 0x53),
            ('t', 0x14, 0x54),
            ('u', 0x16, 0x55),
            ('v', 0x2F, 0x56),
            ('w', 0x11, 0x57),
            ('x', 0x2D, 0x58),
            ('y', 0x15, 0x59),
            ('z', 0x2C, 0x5A),
        ];
        for (ch, scan_code, vk) in letter_keys.iter() {
            let key = ch.to_string();
            map.insert(Box::leak(key.into_boxed_str()), (*scan_code, *vk));
        }

        // Number keys 0-9
        let digit_keys = [
            ('0', 0x0B, 0x30),
            ('1', 0x02, 0x31),
            ('2', 0x03, 0x32),
            ('3', 0x04, 0x33),
            ('4', 0x05, 0x34),
            ('5', 0x06, 0x35),
            ('6', 0x07, 0x36),
            ('7', 0x08, 0x37),
            ('8', 0x09, 0x38),
            ('9', 0x0A, 0x39),
        ];
        for (ch, scan_code, vk) in digit_keys.iter() {
            let key = ch.to_string();
            map.insert(Box::leak(key.into_boxed_str()), (*scan_code, *vk));
        }

        // Function keys F1-F12
        let func_keys = [
            ("f1", 0x3B, 0x70),
            ("f2", 0x3C, 0x71),
            ("f3", 0x3D, 0x72),
            ("f4", 0x3E, 0x73),
            ("f5", 0x3F, 0x74),
            ("f6", 0x40, 0x75),
            ("f7", 0x41, 0x76),
            ("f8", 0x42, 0x77),
            ("f9", 0x43, 0x78),
            ("f10", 0x44, 0x79),
            ("f11", 0x57, 0x7A),
            ("f12", 0x58, 0x7B),
        ];
        for (key, scan_code, vk) in func_keys.iter() {
            map.insert(*key, (*scan_code, *vk));
        }

        map
    });

    KEY_MAP
        .get(&name.to_lowercase().as_str())
        .copied()
        .ok_or_else(|| anyhow::anyhow!("Unknown key name: {}", name))
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
    fn invalidate(&self, instance_id: u32) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.remove(&instance_id);
            debug!("Invalidated config path cache for instance {}", instance_id);
        }
    }

    /// Clear all cache
    fn clear(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
            debug!("Cleared all config path cache");
        }
    }

    /// Internal path resolution logic (original implementation)
    fn resolve_config_path_internal(instance_id: u32) -> Option<std::path::PathBuf> {
        let home = std::env::var("USERPROFILE").ok()?;
        let home_path = std::path::PathBuf::from(home);

        let config_filename = if instance_id == 0 {
            ".wakem.toml".to_string()
        } else {
            format!(".wakem-instance{}.toml", instance_id)
        };

        // Priority 1: Check %USERPROFILE%\.wakem.toml or .wakem-instanceN.toml
        let config_file = home_path.join(&config_filename);
        if config_file.exists() {
            return Some(config_file);
        }

        // Priority 2: Check %APPDATA%\wakem\config.toml or config-instanceN.toml
        let app_data = std::env::var("APPDATA").ok()?;
        let config_dir = std::path::PathBuf::from(app_data).join("wakem");
        let config_file = if instance_id == 0 {
            config_dir.join("config.toml")
        } else {
            config_dir.join(format!("config-instance{}.toml", instance_id))
        };
        if config_file.exists() {
            return Some(config_file);
        }

        // Return default path (even if it doesn't exist)
        Some(home_path.join(config_filename))
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
pub fn invalidate_config_path_cache(instance_id: u32) {
    CONFIG_PATH_CACHE.invalidate(instance_id);
}

/// Clear all config file path cache
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
            // Should have 6 actions: Ctrl press, Alt press, Win press, Win release, Alt release, Ctrl release
            assert_eq!(actions.len(), 6);

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

            // Verify fourth, fifth, sixth actions are release
            if let crate::types::Action::Key(crate::types::KeyAction::Release {
                virtual_key,
                ..
            }) = &actions[3]
            {
                assert_eq!(*virtual_key, 0x5B); // VK_LWIN release
            } else {
                panic!(
                    "Expected Win Release as fourth action, got {:?}",
                    actions[3]
                );
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

        // * 通配符（匹配任意字符序列）
        assert!(wildcard_match_dp("test.exe", "*.exe"));
        assert!(wildcard_match_dp("file.txt", "*.txt"));
        assert!(wildcard_match_dp("", "*"));
        assert!(wildcard_match_dp("anything", "*"));
        assert!(wildcard_match_dp("prefix-suffix", "*suffix"));
        assert!(wildcard_match_dp("prefix-suffix", "prefix*"));

        // ? 通配符（匹配单个字符）
        assert!(wildcard_match_dp("cat", "?at"));
        assert!(wildcard_match_dp("bat", "?at"));
        assert!(!wildcard_match_dp("at", "?at")); // ? 需要一个字符
        assert!(wildcard_match_dp("abc", "???"));
        assert!(!wildcard_match_dp("ab", "???"));

        // 混合使用
        assert!(wildcard_match_dp("test123.txt", "test*.txt"));
        assert!(wildcard_match_dp("file_1.txt", "file_?.txt"));
    }

    #[test]
    fn test_wildcard_dp_edge_cases() {
        // empty string and empty pattern
        assert!(wildcard_match_dp("", ""));
        assert!(!wildcard_match_dp("a", ""));
        assert!(wildcard_match_dp("", "*"));
        assert!(!wildcard_match_dp("", "?")); // ? 需要至少一个字符

        // consecutive *
        assert!(wildcard_match_dp("test", "**test"));
        assert!(wildcard_match_dp("test", "***"));
        assert!(wildcard_match_dp("", "**"));

        // multiple leading *
        assert!(wildcard_match_dp("test", "****test"));

        // case insensitive（已转换为小写）
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
        // Test不会因为长输入而崩溃或栈溢出
        let long_text = "a".repeat(1000);
        let long_pattern = "*".repeat(100);

        // 应该能正常处理，不会栈溢出
        let result = wildcard_match_dp(&long_text, &long_pattern);
        assert!(result); // * 匹配任何内容

        // 空模式和长文本
        assert!(!wildcard_match_dp(&long_text, ""));

        // 长文本和简单模式
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

        // Test带 Win 键
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
        // 空快捷键
        assert!(parse_shortcut_trigger("").is_err());

        // 只有修饰键
        assert!(parse_shortcut_trigger("Ctrl+Alt").is_err());
    }

    #[test]
    fn test_parse_launch_mapping() {
        let rule = parse_launch_mapping("F1", "notepad.exe").unwrap();

        // 验证触发器
        if let crate::types::Trigger::Key { virtual_key, .. } = &rule.trigger {
            assert_eq!(*virtual_key, Some(0x70)); // VK_F1
        } else {
            panic!("Expected Key trigger");
        }

        // 验证动作
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

/// 解析启动项映射
/// 支持格式:
/// - 简单命令: "notepad.exe"
/// - 带参数命令: "notepad.exe C:\\Users\\test.txt"
fn parse_launch_mapping(trigger: &str, command: &str) -> anyhow::Result<MappingRule> {
    use crate::types::{Action, Trigger};

    // 解析触发键
    let (scan_code, virtual_key) = parse_key(trigger)?;
    let trigger_obj = Trigger::key(scan_code, virtual_key);

    // 解析启动命令
    let action = if command.contains(' ') {
        // 使用 Launcher::parse_command 解析带参数的命令
        Action::Launch(Launcher::parse_command(command))
    } else {
        // 简单命令
        Action::launch(command)
    };

    Ok(MappingRule::new(trigger_obj, action))
}
