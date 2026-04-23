use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, trace, warn};

use crate::config::Config;
use crate::constants::{
    INPUT_BATCH_SIZE_LIMIT, INPUT_BATCH_TIMEOUT_MICROS, INPUT_CHANNEL_CAPACITY,
    IPC_CHANNEL_CAPACITY, SHUTDOWN_WAIT_DELAY_MS, WINDOW_EVENT_CHANNEL_CAPACITY,
    WINDOW_PRESET_APPLY_DELAY_MS,
};
use crate::ipc::{IpcServer, Message};
use crate::platform::traits::OutputDeviceTrait;
use crate::runtime::macro_player::MacroPlayer;
use crate::shutdown::ShutdownSignal;
use crate::types::{
    macros::MacroRecorder, Action, InputEvent, KeyState, Macro, ModifierState,
};

use crate::runtime::{KeyMapper, LayerManager};

// Platform-specific imports for production code
#[cfg(all(target_os = "windows", not(test)))]
use crate::platform::windows::{
    Launcher, LegacyRawInputDevice as RawInputDevice, WindowManager,
    WindowPresetManager, WindowsOutputDevice as OutputDevice,
};

#[cfg(all(target_os = "windows", test))]
use crate::platform::windows::{
    Launcher, LegacyRawInputDevice as RawInputDevice, MockOutputDevice as OutputDevice,
    WindowManager, WindowPresetManager,
};

#[cfg(target_os = "macos")]
use crate::platform::macos::input_device::{InputDevice, InputDeviceConfig};

// Platform-specific imports for production code (macOS)
#[cfg(all(target_os = "macos", not(test)))]
use crate::platform::macos::{
    AppCommand as TrayAppCommand, Launcher, MacosInputDevice as RawInputDevice,
    MacosOutputDevice as OutputDevice, RealMacosWindowApi,
    RealMacosWindowManager as WindowManager,
};

// Platform-specific imports for test code (macOS)
#[cfg(all(target_os = "macos", test))]
use crate::platform::macos::{
    output_device::MockMacosOutputDevice as OutputDevice, AppCommand as TrayAppCommand,
    Launcher, MacosInputDevice as RawInputDevice, RealMacosWindowApi,
    RealMacosWindowManager as WindowManager,
};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

/// Server state
///
/// Performance optimization notes:
/// - Use RwLock instead of Mutex (for read-heavy scenarios)
/// - Group related states to reduce lock count
/// - Use Arc to share config and rules to avoid repeated cloning
pub struct ServerState {
    /// Current configuration (read-heavy, write-rare)
    config: Arc<RwLock<Config>>,
    /// Key mapping engine (read-heavy: read on every event, write only on config change)
    mapper: Arc<RwLock<KeyMapper>>,
    /// Layer manager (read-heavy)
    layer_manager: Arc<RwLock<LayerManager>>,
    /// Output device (write-heavy: every action needs to write)
    output_device: Arc<Mutex<OutputDevice>>,
    /// Program launcher (write-heavy: needs mutex when launching programs)
    launcher: Arc<Mutex<Launcher>>,
    /// Window preset manager (balanced read/write) - Windows only
    #[cfg(target_os = "windows")]
    window_preset_manager: Arc<RwLock<WindowPresetManager>>,
    /// Window manager - macOS only
    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    window_manager: Arc<RwLock<crate::platform::macos::RealMacosWindowManager>>,
    /// Whether mapping is enabled (frequently read, rarely written)
    active: Arc<RwLock<bool>>,
    /// Whether config has been loaded
    config_loaded: Arc<RwLock<bool>>,
    /// Macro recorder (has internal synchronization)
    macro_recorder: Arc<MacroRecorder>,
    /// Message window handle (for sending notifications)
    /// Stored as isize for Send/Sync safety (HWND is *mut c_void which is not Send)
    #[allow(dead_code)]
    message_window_hwnd: Arc<RwLock<Option<isize>>>,
    /// Auth key (stored separately, supports dynamic updates)
    auth_key: Arc<RwLock<String>>,
    /// Virtual modifier state for Hyper key support
    /// This tracks modifiers injected by hyper keys since GetAsyncKeyState
    /// cannot reliably detect injected key states
    virtual_modifiers: Arc<RwLock<ModifierState>>,
    /// Hyper key mapping: (scan_code, virtual_key) -> modifiers to inject
    /// Dynamically extracted from remap rules where target is a modifier combo
    hyper_key_map: Arc<RwLock<std::collections::HashMap<(u16, u16), ModifierState>>>,
}

impl ServerState {
    #[cfg(target_os = "windows")]
    pub fn new() -> Self {
        let window_manager = WindowManager::new();
        let mut mapper = KeyMapper::with_window_manager(window_manager);
        let window_preset_manager = WindowPresetManager::new();
        mapper.set_window_preset_manager(window_preset_manager);

        Self {
            config: Arc::new(RwLock::new(Config::default())),
            mapper: Arc::new(RwLock::new(mapper)),
            layer_manager: Arc::new(RwLock::new(LayerManager::new())),
            output_device: Arc::new(Mutex::new(OutputDevice::new())),
            launcher: Arc::new(Mutex::new(Launcher::new())),
            window_preset_manager: Arc::new(RwLock::new(WindowPresetManager::new())),
            active: Arc::new(RwLock::new(true)),
            config_loaded: Arc::new(RwLock::new(false)),
            macro_recorder: Arc::new(MacroRecorder::new()),
            message_window_hwnd: Arc::new(RwLock::new(None)),
            auth_key: Arc::new(RwLock::new(String::new())),
            virtual_modifiers: Arc::new(RwLock::new(ModifierState::new())),
            hyper_key_map: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn new() -> Self {
        let window_manager = WindowManager::new(RealMacosWindowApi::new());
        let mapper = KeyMapper::with_window_manager(window_manager);

        Self {
            config: Arc::new(RwLock::new(Config::default())),
            mapper: Arc::new(RwLock::new(mapper)),
            layer_manager: Arc::new(RwLock::new(LayerManager::new())),
            output_device: Arc::new(Mutex::new(OutputDevice::new())),
            launcher: Arc::new(Mutex::new(Launcher::new())),
            window_manager: Arc::new(RwLock::new(WindowManager::new(
                RealMacosWindowApi::new(),
            ))),
            active: Arc::new(RwLock::new(true)),
            config_loaded: Arc::new(RwLock::new(false)),
            macro_recorder: Arc::new(MacroRecorder::new()),
            message_window_hwnd: Arc::new(RwLock::new(None)),
            auth_key: Arc::new(RwLock::new(String::new())),
            virtual_modifiers: Arc::new(RwLock::new(ModifierState::new())),
            hyper_key_map: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Load configuration
    ///
    /// Performance optimization: batch updates to reduce lock hold time
    #[tracing::instrument(skip(self, config), fields(
        rules_count = config.get_all_rules().len(),
        layers_count = config.keyboard.layers.len(),
        presets_count = config.window.presets.len(),
        context_mappings_count = config.keyboard.context_mappings.len(),
    ))]
    pub async fn load_config(&self, config: Config) -> Result<()> {
        // Debug: Log config details
        debug!(
            window_shortcuts_count = config.window.shortcuts.len(),
            launch_count = config.launch.len(),
            "Config details"
        );
        if !config.window.shortcuts.is_empty() {
            for (key, value) in &config.window.shortcuts {
                debug!(shortcut_key = %key, shortcut_value = %value, "Window shortcut");
            }
        }

        // 1. Update auth key (stored separately from config)
        {
            let mut key = self.auth_key.write().await;
            *key = config.network.auth_key.clone().unwrap_or_default();
        }

        // 2. Update base mapping rules and context rules (merged into one write lock)
        {
            let mut mapper = self.mapper.write().await;
            let rules = config.get_all_rules();
            mapper.load_rules(rules);
            mapper.load_context_rules(&config.keyboard.context_mappings);
            debug!(
                context_mappings_count = config.keyboard.context_mappings.len(),
                "Loaded context mappings"
            );
        }

        // 2.5. Extract hyper key mappings from remap config
        {
            let hyper_map = config.get_hyper_key_mappings();
            let mut hyper_key_map = self.hyper_key_map.write().await;
            *hyper_key_map = hyper_map;
            debug!(
                hyper_key_count = hyper_key_map.len(),
                "Loaded hyper key mappings"
            );
        }

        // 3. Update window preset manager (Windows only)
        #[cfg(target_os = "windows")]
        {
            let mut preset_manager = self.window_preset_manager.write().await;
            preset_manager.load_presets(config.window.presets.clone());
            debug!(
                presets_count = config.window.presets.len(),
                "Loaded window presets"
            );
        }

        // 4. Update layer manager
        {
            let mut layer_manager = self.layer_manager.write().await;

            // Load base mappings
            let base_rules = config.get_all_rules();
            layer_manager.set_base_mappings(base_rules);

            // Load layer configs
            for (name, layer_config) in &config.keyboard.layers {
                let mode = match layer_config.mode {
                    crate::config::LayerMode::Hold => crate::types::LayerMode::Hold,
                    crate::config::LayerMode::Toggle => crate::types::LayerMode::Toggle,
                };
                let mappings: Vec<(String, String)> = layer_config
                    .mappings
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                match LayerManager::create_layer_from_config(
                    name,
                    &layer_config.activation_key,
                    mode,
                    &mappings,
                ) {
                    Ok(layer) => {
                        layer_manager.register_layer(layer);
                        info!("Registered layer: {}", name);
                    }
                    Err(e) => {
                        error!("Failed to create layer {}: {}", name, e);
                    }
                }
            }
        }

        // 5. Finally update config (ensure all components are ready)
        {
            let mut cfg = self.config.write().await;
            *cfg = config;
        }

        // 6. Mark config as loaded
        {
            let mut loaded = self.config_loaded.write().await;
            *loaded = true;
        }

        info!("Configuration loaded successfully");
        Ok(())
    }

    /// Reload configuration from file
    pub async fn reload_config_from_file(&self) -> Result<()> {
        use crate::config::resolve_config_file_path;

        info!("Reloading configuration from file...");

        // Get current instance ID and config file path
        let (_instance_id, config_path) = {
            let config = self.config.read().await;
            let id = config.network.instance_id;
            let path = resolve_config_file_path(None, id);
            (id, path)
        };

        let config_path = match config_path {
            Some(path) => path,
            None => {
                return Err(anyhow::anyhow!("Config file not found"));
            }
        };

        info!("Loading config from: {:?}", config_path);

        // Try to load new config
        let new_config = match Config::from_file(&config_path) {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load config: {}", e);
                return Err(anyhow::anyhow!("Failed to load config: {}", e));
            }
        };

        // Apply new config
        self.load_config(new_config).await?;

        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Save current configuration to file
    pub async fn save_config_to_file(&self) -> Result<()> {
        use crate::config::resolve_config_file_path;

        info!("Saving configuration to file...");

        // Get current instance ID and config file path
        let (_instance_id, config_path) = {
            let config = self.config.read().await;
            let id = config.network.instance_id;
            let path = resolve_config_file_path(None, id);
            (id, path)
        };

        let config_path = match config_path {
            Some(path) => path,
            None => {
                return Err(anyhow::anyhow!("Config file path not found"));
            }
        };

        info!("Saving config to: {:?}", config_path);

        // Get current config and save
        let config = self.config.read().await;
        match config.save_to_file(&config_path) {
            Ok(_) => {
                info!("Configuration saved successfully");
                Ok(())
            }
            Err(e) => {
                error!("Failed to save config: {}", e);
                Err(anyhow::anyhow!("Failed to save config: {}", e))
            }
        }
    }

    /// Process input event
    ///
    /// Performance optimizations:
    /// - Use RwLock.read() instead of Mutex.lock() (for read-heavy scenarios)
    /// - Reduce lock hold time, fast path returns early
    /// - Batch read related states
    #[tracing::instrument(skip(self, event), fields(event_type = %event.event_type_name()))]
    pub async fn process_input_event(&self, event: InputEvent) {
        // Fast path: check if enabled (lightest lock)
        if !*self.active.read().await {
            return;
        }

        // If injected event, ignore (to avoid loops)
        if event.is_injected() {
            return;
        }

        // If recording macro, record event (read operation, quick release lock)
        if self.macro_recorder.is_recording().await {
            self.macro_recorder.record_event(&event).await;
        }

        // Process wheel enhancement (read-only config, no state modification)
        if let InputEvent::Mouse(mouse_event) = &event {
            if let crate::types::MouseEventType::Wheel(delta) = mouse_event.event_type {
                debug!(wheel_delta = delta, "Processing wheel enhancement");
                if let Some(action) = self.process_wheel_enhancement(delta).await {
                    if let Err(e) = self.execute_action(action).await {
                        error!(
                            error = %e,
                            wheel_delta = delta,
                            "Failed to execute wheel action"
                        );
                    }
                    return;
                }
            }
        }

        // Check if this is CapsLock (Hyper key) and update virtual modifiers directly
        // This must happen before merge_virtual_modifiers
        let _is_hyper_key = self.check_and_update_hyper_key(&event).await;

        // Merge virtual modifiers into the event for Hyper key support
        let event = self.merge_virtual_modifiers(event).await;

        // Filter out key release events for non-hyper keys
        // This replaces the old RI_KEY_BREAK filter in input.rs which dropped ALL key-up events.
        // We need hyper-key releases to pass through (to clear virtual_modifiers),
        // but must block other releases to prevent double-triggering of shortcut actions.
        if let InputEvent::Key(ref key_event) = event {
            if key_event.state == KeyState::Released {
                let hyper_map = self.hyper_key_map.read().await;
                if !hyper_map.contains_key(&(key_event.scan_code, key_event.virtual_key))
                {
                    return;
                }
            }
        }

        // First try to process through layer manager (optimization: reduce write lock hold time)
        let (handled, action) = {
            let mut layer_manager = self.layer_manager.write().await;
            layer_manager.process_event(&event)
        };

        if handled {
            if let Some(action) = action {
                if let Err(e) = self.execute_action(action).await {
                    error!("Failed to execute action: {}", e);
                }
            }
            return;
        }

        // Layer manager didn't handle, use base mapping engine (with context awareness) - use read lock
        let action = {
            let mapper = self.mapper.read().await;
            #[cfg(target_os = "windows")]
            let context: Option<crate::platform::traits::WindowContext> =
                crate::platform::windows::WindowContext::get_current().map(|ctx| {
                    crate::platform::traits::WindowContext {
                        process_name: ctx.process_name,
                        window_class: ctx.window_class,
                        window_title: ctx.window_title,
                        executable_path: if ctx.executable_path.is_empty() {
                            None
                        } else {
                            Some(ctx.executable_path)
                        },
                    }
                });
            #[cfg(target_os = "macos")]
            let context: Option<crate::platform::traits::WindowContext> =
                crate::platform::macos::WindowContext::get_current();
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            let context: Option<crate::platform::traits::WindowContext> = None;
            mapper.process_event_with_context(&event, context.as_ref())
        };

        // Execute action (outside lock to avoid long lock hold)
        if let Some(action) = action {
            if let Err(e) = self.execute_action(action).await {
                error!("Failed to execute action: {}", e);
            }
        }
    }

    /// Check if this is a hyper key and update virtual modifiers
    /// A hyper key is any key remapped to a modifier combination (e.g., CapsLock -> Ctrl+Alt+Meta)
    /// Returns true if this is a hyper key event
    async fn check_and_update_hyper_key(&self, event: &InputEvent) -> bool {
        if let InputEvent::Key(key_event) = event {
            let hyper_key_map = self.hyper_key_map.read().await;
            if let Some(&modifiers) =
                hyper_key_map.get(&(key_event.scan_code, key_event.virtual_key))
            {
                drop(hyper_key_map);
                let mut virtual_mods = self.virtual_modifiers.write().await;
                match key_event.state {
                    crate::types::KeyState::Pressed => {
                        virtual_mods.shift |= modifiers.shift;
                        virtual_mods.ctrl |= modifiers.ctrl;
                        virtual_mods.alt |= modifiers.alt;
                        virtual_mods.meta |= modifiers.meta;
                        debug!(
                            scan_code = key_event.scan_code,
                            vk = key_event.virtual_key,
                            ?modifiers,
                            "Hyper key pressed, virtual modifiers activated"
                        );
                    }
                    crate::types::KeyState::Released => {
                        *virtual_mods = ModifierState::new();
                        debug!(
                            scan_code = key_event.scan_code,
                            vk = key_event.virtual_key,
                            "Hyper key released, virtual modifiers cleared"
                        );
                    }
                }
                return true;
            }
        }
        false
    }

    /// Merge virtual modifiers into key event for hyper key support
    /// Skips the hyper key itself so it can match its own remap rule with original modifiers
    async fn merge_virtual_modifiers(&self, mut event: InputEvent) -> InputEvent {
        if let InputEvent::Key(ref mut key_event) = event {
            let hyper_key_map = self.hyper_key_map.read().await;
            if hyper_key_map.contains_key(&(key_event.scan_code, key_event.virtual_key))
            {
                return event;
            }
            drop(hyper_key_map);

            let virtual_mods = *self.virtual_modifiers.read().await;
            key_event.modifiers.shift |= virtual_mods.shift;
            key_event.modifiers.ctrl |= virtual_mods.ctrl;
            key_event.modifiers.alt |= virtual_mods.alt;
            key_event.modifiers.meta |= virtual_mods.meta;
            InputEvent::Key(key_event.clone())
        } else {
            event
        }
    }

    /// Process wheel enhancement
    async fn process_wheel_enhancement(&self, delta: i32) -> Option<Action> {
        let config = self.config.read().await;
        let wheel_config = &config.mouse.wheel;

        // Get current modifier state
        let modifiers = get_current_modifier_state();

        // Check volume control
        if let Some(volume_config) = &wheel_config.volume_control {
            if Self::check_modifier_match(&volume_config.modifier, &modifiers) {
                if delta > 0 {
                    return Some(Action::System(crate::types::SystemAction::VolumeUp));
                } else {
                    return Some(Action::System(crate::types::SystemAction::VolumeDown));
                }
            }
        }

        // Check brightness control
        if let Some(brightness_config) = &wheel_config.brightness_control {
            if Self::check_modifier_match(&brightness_config.modifier, &modifiers) {
                if delta > 0 {
                    return Some(Action::System(
                        crate::types::SystemAction::BrightnessUp,
                    ));
                } else {
                    return Some(Action::System(
                        crate::types::SystemAction::BrightnessDown,
                    ));
                }
            }
        }

        // Check horizontal scroll
        if let Some(hscroll_config) = &wheel_config.horizontal_scroll {
            if Self::check_modifier_match(&hscroll_config.modifier, &modifiers) {
                // Convert vertical wheel to horizontal wheel
                return Some(Action::Mouse(crate::types::MouseAction::HWheel {
                    delta: delta * hscroll_config.step,
                }));
            }
        }

        // Check wheel acceleration
        if wheel_config.acceleration {
            // Simple acceleration implementation: increase scroll distance based on direction
            let accelerated_delta = delta * wheel_config.acceleration_multiplier as i32;
            return Some(Action::Mouse(crate::types::MouseAction::Wheel {
                delta: accelerated_delta,
            }));
        }

        None
    }

    /// Check if modifier key matches
    fn check_modifier_match(modifier_str: &str, modifiers: &ModifierState) -> bool {
        match modifier_str.to_lowercase().as_str() {
            "shift" => modifiers.shift,
            "ctrl" | "control" => modifiers.ctrl,
            "alt" => modifiers.alt,
            "win" | "meta" | "command" => modifiers.meta,
            "rightalt" => modifiers.alt, // Simplified handling
            "rightctrl" => modifiers.ctrl, // Simplified handling
            "rightshift" => modifiers.shift, // Simplified handling
            _ => false,
        }
    }

    /// Execute action
    ///
    /// Performance optimizations:
    /// - Group Sequence actions to reduce lock acquisition count
    /// - Consecutive Key/Mouse/System actions share the same output_device lock
    /// - Window/Launch actions use separate locks to avoid blocking other operations for too long
    async fn execute_action(&self, action: Action) -> Result<()> {
        debug!(action_type = ?action, "Executing action");
        match action {
            Action::Key(key_action) => {
                let output = self.output_device.lock().await;
                output.send_key_action(&key_action)?;
            }
            Action::Mouse(mouse_action) => {
                let output = self.output_device.lock().await;
                output.send_mouse_action(&mouse_action)?;
            }
            Action::Window(window_action) => {
                info!(?window_action, "Executing window action");
                let mut mapper = self.mapper.write().await;
                #[cfg(target_os = "macos")]
                {
                    debug!(
                        has_window_manager = mapper.window_manager.is_some(),
                        "Checking window manager availability"
                    );
                }
                mapper.execute_action(&Action::Window(window_action))?;
                info!("Window action executed successfully");
            }
            Action::Launch(launch_action) => {
                let launcher = self.launcher.lock().await;
                launcher.launch(&launch_action)?;
            }
            Action::Sequence(actions) => {
                // Performance optimization: group action sequence execution to reduce lock acquisition
                self.execute_action_sequence_optimized(&actions).await?;
            }
            Action::System(system_action) => {
                let output = self.output_device.lock().await;
                output.send_system_action(&system_action)?;
            }
            Action::Delay { milliseconds } => {
                tokio::time::sleep(tokio::time::Duration::from_millis(milliseconds))
                    .await;
            }
            Action::None => {}
        }

        Ok(())
    }

    /// Optimized action sequence execution (reduces lock contention)
    ///
    /// Group actions in sequence by type:
    /// - Consecutive Key/Mouse/System actions: acquire output_device lock once and execute in batch
    /// - Window actions: acquire mapper write lock separately
    /// - Launch actions: acquire launcher lock separately
    /// - Delay actions: wait after releasing all locks
    async fn execute_action_sequence_optimized(&self, actions: &[Action]) -> Result<()> {
        use crate::types::Action::*;

        let mut i = 0;
        while i < actions.len() {
            match &actions[i] {
                // Batch process output device related actions (Key, Mouse, System)
                Key(_) | Mouse(_) | System(_) => {
                    let output = self.output_device.lock().await;

                    // Collect all consecutive output device actions
                    while i < actions.len() {
                        match &actions[i] {
                            Key(key_action) => {
                                output.send_key_action(key_action)?;
                            }
                            Mouse(mouse_action) => {
                                output.send_mouse_action(mouse_action)?;
                            }
                            System(system_action) => {
                                output.send_system_action(system_action)?;
                            }
                            _ => break, // Encountered non-output device action, stop batch processing
                        }
                        i += 1;
                    }
                    // output lock released here
                }

                // Handle window actions separately
                Window(window_action) => {
                    let mut mapper = self.mapper.write().await;
                    mapper.execute_action(&Window(window_action.clone()))?;
                    i += 1;
                    // mapper lock released here
                }

                // Handle launch actions separately
                Launch(launch_action) => {
                    let launcher = self.launcher.lock().await;
                    launcher.launch(launch_action)?;
                    i += 1;
                    // launcher lock released here
                }

                // Handle delay actions (wait without locks)
                Delay { milliseconds } => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        *milliseconds,
                    ))
                    .await;
                    i += 1;
                }

                // No-op
                None => {
                    i += 1;
                }

                // Nested Sequence (process recursively)
                Sequence(nested_actions) => {
                    Box::pin(self.execute_action_sequence_optimized(nested_actions))
                        .await?;
                    i += 1;
                }
            }
        }

        Ok(())
    }

    /// Set active state
    pub async fn set_active(&self, active: bool) {
        let mut a = self.active.write().await;
        *a = active;
        info!("Server active state: {}", active);
    }

    /// Get status
    pub async fn get_status(&self) -> (bool, bool) {
        (*self.active.read().await, *self.config_loaded.read().await)
    }

    /// Start macro recording
    pub async fn start_macro_recording(&self, name: &str) -> Result<()> {
        self.macro_recorder.start_recording(name).await
    }

    /// Stop macro recording
    pub async fn stop_macro_recording(&self) -> Result<Macro> {
        let macro_def = self.macro_recorder.stop_recording().await?;
        self.save_macro(&macro_def).await?;

        // Show recording complete notification
        let step_count = macro_def.steps.len();
        let _ = self
            .show_notification(
                "wakem - Macro Recording",
                &format!(
                    "Macro '{}' recording completed with {} steps",
                    macro_def.name, step_count
                ),
            )
            .await;

        Ok(macro_def)
    }

    /// Save macro to config
    async fn save_macro(&self, macro_def: &Macro) -> Result<()> {
        let mut config = self.config.write().await;
        config
            .macros
            .insert(macro_def.name.clone(), macro_def.steps.clone());

        // Try to save to file, but don't fail if it doesn't work (e.g., in tests)
        if let Some(config_path) =
            crate::config::resolve_config_file_path(None, config.network.instance_id)
        {
            if let Err(e) = config.save_to_file(&config_path) {
                warn!("Failed to save config to file: {}", e);
                // Continue even if file save fails - the macro is still in memory
            }
        }

        info!("Macro '{}' saved to config", macro_def.name);
        Ok(())
    }

    /// Play macro
    pub async fn play_macro(&self, name: &str) -> Result<()> {
        let config = self.config.read().await;
        let steps = config
            .macros
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Macro '{}' not found", name))?
            .clone();

        let macro_def = Macro {
            name: name.to_string(),
            steps,
            created_at: None,
            description: None,
        };

        drop(config); // Release read lock

        let output_device = self.output_device.lock().await;
        MacroPlayer::play_macro(&output_device, &macro_def).await?;

        // Show playback complete notification
        let _ = self
            .show_notification(
                "wakem - Macro Playback",
                &format!("Macro '{}' playback completed", name),
            )
            .await;

        Ok(())
    }

    /// Get macro list
    pub async fn get_macros(&self) -> Vec<String> {
        let config = self.config.read().await;
        config.macros.keys().cloned().collect()
    }

    /// Delete macro
    pub async fn delete_macro(&self, name: &str) -> Result<()> {
        let mut config = self.config.write().await;
        if config.macros.remove(name).is_none() {
            return Err(anyhow::anyhow!("Macro '{}' not found", name));
        }

        // Also remove bindings
        config.macro_bindings.retain(|_, v| v != name);

        // Save to file
        let config_path =
            crate::config::resolve_config_file_path(None, config.network.instance_id)
                .ok_or_else(|| anyhow::anyhow!("Config path not found"))?;
        config.save_to_file(&config_path)?;

        info!("Macro '{}' deleted", name);
        Ok(())
    }

    /// Bind macro to trigger key
    pub async fn bind_macro(&self, macro_name: &str, trigger: &str) -> Result<()> {
        let mut config = self.config.write().await;

        // Check if macro exists
        if !config.macros.contains_key(macro_name) {
            return Err(anyhow::anyhow!("Macro '{}' not found", macro_name));
        }

        // Add binding
        config
            .macro_bindings
            .insert(trigger.to_string(), macro_name.to_string());

        // Save to file
        let config_path =
            crate::config::resolve_config_file_path(None, config.network.instance_id)
                .ok_or_else(|| anyhow::anyhow!("Config path not found"))?;
        config.save_to_file(&config_path)?;

        info!("Macro '{}' bound to '{}'", macro_name, trigger);
        Ok(())
    }

    /// Check if recording macro
    #[allow(dead_code)]
    pub async fn is_recording_macro(&self) -> bool {
        self.macro_recorder.is_recording().await
    }

    /// Set message window handle
    /// Takes isize instead of HWND because HWND is not Send and cannot be used across await points
    #[cfg(target_os = "windows")]
    pub async fn set_message_window_hwnd(&self, hwnd_value: isize) {
        let mut h = self.message_window_hwnd.write().await;
        *h = Some(hwnd_value);
        info!(
            "Message window handle registered: {:?}",
            HWND(hwnd_value as *mut std::ffi::c_void)
        );
    }

    /// Set message window handle (macOS version - no-op)
    #[cfg(target_os = "macos")]
    pub async fn set_message_window_hwnd(&self, _hwnd_value: isize) {
        // macOS doesn't use HWND, this is a no-op
        info!("Message window handle registered (macOS)");
    }

    /// Get current auth key (for IPC authentication)
    #[allow(dead_code)]
    pub async fn get_auth_key(&self) -> String {
        self.auth_key.read().await.clone()
    }

    /// Show tray notification
    #[cfg(target_os = "windows")]
    pub async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        if let Some(hwnd_isize) = *self.message_window_hwnd.read().await {
            // Show notification using tray icon (pass isize directly)
            self.show_tray_notification(hwnd_isize, title, message)
                .await?;
        } else {
            debug!("Message window not registered, skipping notification");
        }
        Ok(())
    }

    /// Show tray notification (macOS version)
    #[cfg(target_os = "macos")]
    pub async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        use crate::platform::macos::native_api::notification::show_notification;

        match show_notification(title, message) {
            Ok(()) => {
                info!("Notification shown: {} - {}", title, message);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to show notification: {}", e);
                Ok(())
            }
        }
    }

    /// Show notification using tray icon (internal method)
    /// Takes isize instead of HWND to avoid Send issues
    #[cfg(target_os = "windows")]
    async fn show_tray_notification(
        &self,
        hwnd_value: isize,
        title: &str,
        message: &str,
    ) -> Result<()> {
        use windows::Win32::UI::Shell::{
            NIF_INFO, NIM_MODIFY, NOTIFYICONDATAW, NOTIFY_ICON_INFOTIP_FLAGS,
        };

        // Create HWND from isize for API calls
        let hwnd = HWND(hwnd_value as *mut std::ffi::c_void);

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1, // Tray icon ID
            uFlags: NIF_INFO,
            ..Default::default()
        };

        // Convert title and message to wide strings
        let title_wide: Vec<u16> =
            title.encode_utf16().chain(std::iter::once(0)).collect();
        let message_wide: Vec<u16> =
            message.encode_utf16().chain(std::iter::once(0)).collect();

        // Copy to struct (limit length)
        let title_len = title_wide.len().min(64);
        let message_len = message_wide.len().min(256);

        nid.szInfoTitle[..title_len].copy_from_slice(&title_wide[..title_len]);
        nid.szInfo[..message_len].copy_from_slice(&message_wide[..message_len]);

        // Set notification type (0 = no icon)
        nid.dwInfoFlags = NOTIFY_ICON_INFOTIP_FLAGS(0);

        unsafe {
            let result = windows::Win32::UI::Shell::Shell_NotifyIconW(NIM_MODIFY, &nid);
            if !result.as_bool() {
                return Err(anyhow::anyhow!("Failed to show notification"));
            }
        }

        info!("Notification shown: {} - {}", title, message);
        Ok(())
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current modifier key state
fn get_current_modifier_state() -> ModifierState {
    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::get_modifier_state()
    }

    #[cfg(target_os = "macos")]
    {
        crate::platform::macos::get_modifier_state()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        ModifierState::default()
    }
}

/// Run server
///
/// Improvement: integrated graceful shutdown mechanism, supports safe exit of all background tasks
pub async fn run_server(instance_id: u32) -> Result<()> {
    info!("Starting wakemd server (instance {})...", instance_id);

    let state = Arc::new(ServerState::new());

    // Create graceful shutdown signal
    let shutdown = Arc::new(ShutdownSignal::new());
    let shutdown_for_tasks = shutdown.subscribe();

    // Set instance ID
    {
        let mut config = state.config.write().await;
        config.network.instance_id = instance_id;
    }

    // Load configuration from file on startup
    if let Err(e) = state.reload_config_from_file().await {
        warn!(
            "Failed to load config on startup: {}. Using default config.",
            e
        );
    } else {
        info!("Configuration loaded successfully on startup");
    }

    // Create IPC server (with dynamic auth key)
    let (message_tx, mut message_rx) = mpsc::channel(IPC_CHANNEL_CAPACITY);
    let bind_address = {
        let mut config = state.config.write().await;
        let addr = config.network.get_bind_address();
        // Ensure auth key exists (security requirement)
        config.network.ensure_auth_key();
        addr
    };

    info!("Server authentication enabled with dynamic key updates");

    let mut ipc_server = IpcServer::new_with_dynamic_key(
        bind_address.clone(),
        state.auth_key.clone(),
        message_tx.clone(),
    );
    ipc_server.start().await?;

    info!("Server listening on {}", bind_address);

    // Create input event channel (using tokio::sync::mpsc for efficient async processing)
    let (input_tx, mut input_rx) =
        tokio::sync::mpsc::channel::<InputEvent>(INPUT_CHANNEL_CAPACITY);

    // Collect all std thread JoinHandles for graceful shutdown
    let mut thread_handles: Vec<std::thread::JoinHandle<()>> = Vec::new();

    // Start Raw Input capture (in separate thread, send to tokio channel via bridge)
    let input_tx_bridge = input_tx.clone();
    let input_shutdown_flag = Arc::new(AtomicBool::new(false));
    let input_shutdown_flag_clone = input_shutdown_flag.clone();
    let raw_input_shutdown_flag = Arc::new(AtomicBool::new(false));
    let raw_input_shutdown_flag_clone = raw_input_shutdown_flag.clone();

    let input_bridge_handle = std::thread::spawn(move || {
        let (std_tx, std_rx) = std::sync::mpsc::channel::<InputEvent>();
        let tx_clone = input_tx_bridge;
        let shutdown_flag = input_shutdown_flag_clone;
        let raw_input_shutdown = raw_input_shutdown_flag_clone;
        let raw_input_shutdown_for_bridge = raw_input_shutdown.clone();

        #[cfg(target_os = "windows")]
        let raw_input_handle =
            std::thread::spawn(move || match RawInputDevice::new(std_tx) {
                Ok(mut device) => {
                    info!("Raw Input device initialized");
                    // Run until shutdown signal
                    while !raw_input_shutdown.load(Ordering::SeqCst) {
                        if let Err(e) = device.run_once() {
                            error!("Raw Input error: {}", e);
                            break;
                        }
                    }
                    info!("Raw Input thread shutting down");
                }
                Err(e) => {
                    error!("Failed to create Raw Input device: {}", e);
                }
            });

        #[cfg(target_os = "macos")]
        let raw_input_handle = std::thread::spawn(move || {
            match RawInputDevice::new(InputDeviceConfig::default()) {
                Ok(mut device) => {
                    info!("Raw Input device initialized");
                    // Run until shutdown signal
                    while !raw_input_shutdown.load(Ordering::SeqCst) {
                        if let Err(e) = device.run_once() {
                            error!("Raw Input error: {}", e);
                            break;
                        }
                    }
                    info!("Raw Input thread shutting down");
                }
                Err(e) => {
                    error!("Failed to create Raw Input device: {}", e);
                }
            }
        });

        // Bridge: receive from std channel and send to tokio channel
        // Use try_recv to allow checking shutdown flag
        loop {
            if shutdown_flag.load(Ordering::SeqCst) {
                // Signal Raw Input thread to stop
                raw_input_shutdown_for_bridge.store(true, Ordering::SeqCst);
                // Wait for Raw Input thread to finish
                let _ = raw_input_handle.join();
                break;
            }
            match std_rx.try_recv() {
                Ok(event) => {
                    if tx_clone.blocking_send(event).is_err() {
                        // Channel closed, signal Raw Input to stop and exit
                        raw_input_shutdown_for_bridge.store(true, Ordering::SeqCst);
                        let _ = raw_input_handle.join();
                        break;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // No events, sleep briefly to avoid busy waiting
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Sender dropped, signal Raw Input to stop and exit
                    raw_input_shutdown_for_bridge.store(true, Ordering::SeqCst);
                    let _ = raw_input_handle.join();
                    break;
                }
            }
        }
        info!("Input bridge thread shutdown complete");
    });
    thread_handles.push(input_bridge_handle);

    // Store input_shutdown_flag for later use during shutdown
    let input_shutdown_flag_stored = input_shutdown_flag;

    // Start input processing task (with shutdown signal check and batch processing optimization)
    let state_clone = state.clone();
    let mut input_shutdown = shutdown_for_tasks.clone();
    tokio::spawn(async move {
        use tokio::time::{Duration, Instant};

        let batch_size_limit = INPUT_BATCH_SIZE_LIMIT; // Max batch size per processing
        let batch_timeout_micros = INPUT_BATCH_TIMEOUT_MICROS; // Batch timeout (microseconds)
        let mut event_batch = Vec::with_capacity(batch_size_limit);

        loop {
            // Batch collect events (reduce lock contention)
            let batch_start = Instant::now();

            // Try to collect multiple events non-blocking
            loop {
                match input_rx.try_recv() {
                    Ok(event) => {
                        event_batch.push(event);

                        // Stop collecting when batch limit or timeout reached
                        if event_batch.len() >= batch_size_limit {
                            break;
                        }
                        if batch_start.elapsed()
                            >= Duration::from_micros(batch_timeout_micros)
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        // No more events, exit collection loop
                        if !event_batch.is_empty() {
                            break; // Have collected events, start processing
                        }
                        // No events, wait for new events or shutdown signal
                        break;
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        // Channel closed, process remaining events then exit
                        if event_batch.is_empty() {
                            return; // No remaining events, exit directly
                        }
                        break;
                    }
                }

                // If no events collected, use select! to wait
                if event_batch.is_empty() {
                    break;
                }
            }

            // If still no events, wait for new events or shutdown signal
            if event_batch.is_empty() {
                tokio::select! {
                    event = input_rx.recv() => {
                        match event {
                            Some(event) => {
                                // Received single event, process directly
                                state_clone.process_input_event(event).await;
                            }
                            None => break, // Channel closed
                        }
                    }
                    _ = input_shutdown.changed() => {
                        info!("Input processing task received shutdown signal");
                        break;
                    }
                }
            } else {
                // Batch process collected events
                let batch_len = event_batch.len();
                if batch_len > 1 {
                    debug!(batch_size = batch_len, "Processing event batch");
                }

                for event in event_batch.drain(..) {
                    // Check shutdown signal (check between batches)
                    if input_shutdown.has_changed().unwrap_or(false) {
                        info!("Input processing task received shutdown signal during batch");
                        return;
                    }

                    state_clone.process_input_event(event).await;
                }
            }
        }
        info!("Input processing task stopped");
    });

    // Create shutdown flag for window event bridge thread (Windows only)
    #[cfg(target_os = "windows")]
    let window_shutdown_flag = Arc::new(AtomicBool::new(false));
    #[cfg(target_os = "windows")]
    let window_shutdown_flag_clone = window_shutdown_flag.clone();

    // Start window event listener (for auto-applying presets) - Windows only
    #[cfg(target_os = "windows")]
    {
        let mut window_event_rx = {
            let (tx, rx) = tokio::sync::mpsc::channel::<
                crate::platform::windows::WindowEvent,
            >(WINDOW_EVENT_CHANNEL_CAPACITY);

            // Create shutdown flag for window event hook
            let hook_shutdown_flag = Arc::new(AtomicBool::new(false));
            let hook_shutdown_flag_clone = hook_shutdown_flag.clone();
            let shutdown_flag = window_shutdown_flag_clone;

            let window_bridge_handle = std::thread::spawn(move || {
                let (std_tx, std_rx) =
                    std::sync::mpsc::channel::<crate::platform::windows::WindowEvent>();

                let hook_shutdown_flag_inner = hook_shutdown_flag.clone();
                let hook_handle = std::thread::spawn(move || {
                    let mut hook =
                        crate::platform::windows::WindowEventHook::new(std_tx);
                    if let Err(e) = hook.start_with_shutdown(hook_shutdown_flag_inner) {
                        error!("Failed to start window event hook: {}", e);
                    } else {
                        info!("Window event hook started");
                        // Graceful exit: check shutdown flag instead of infinite sleep
                        while !hook.shutdown_flag().load(Ordering::SeqCst) {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        info!("Window event hook received shutdown signal");
                    }
                    hook.stop();
                });

                // Bridge: receive from std channel and send to tokio channel
                // Use try_recv to allow checking shutdown flag
                loop {
                    if shutdown_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    match std_rx.try_recv() {
                        Ok(event) => {
                            if tx.blocking_send(event).is_err() {
                                break;
                            }
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            // No events, sleep briefly to avoid busy waiting
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            // Sender dropped, exit
                            break;
                        }
                    }
                }

                // Signal hook thread to shutdown
                hook_shutdown_flag_clone.store(true, Ordering::SeqCst);

                // Wait for hook thread to finish
                let _ = hook_handle.join();
                info!("Window event bridge thread shutdown complete");
            });
            thread_handles.push(window_bridge_handle);

            rx
        };

        // Start window event handling task (with shutdown signal check)
        let state_clone = state.clone();
        let mut window_shutdown = shutdown_for_tasks.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = window_event_rx.recv() => {
                        match event {
                            Some(event) => {
                                state_clone.handle_window_event(event).await;
                            }
                            None => break,
                        }
                    }
                    _ = window_shutdown.changed() => {
                        info!("Window event handling task received shutdown signal");
                        break;
                    }
                }
            }
            info!("Window event handling task stopped");
        });
    }

    // Start IPC server main loop (with shutdown signal check)
    let mut ipc_shutdown = shutdown_for_tasks.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = ipc_server.run() => {
                    if let Err(e) = result {
                        error!("IPC server error: {}", e);
                        // Wait a short time before retrying after error
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
                _ = ipc_shutdown.changed() => {
                    info!("IPC server received shutdown signal");
                    break;
                }
            }
        }
        info!("IPC server task stopped");
    });

    // Handle IPC messages (with shutdown signal check)
    let state_clone = state.clone();
    let mut msg_handler_shutdown = shutdown_for_tasks.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = message_rx.recv() => {
                    match msg {
                        Some((message, response_tx)) => {
                            let response: crate::ipc::Message = handle_message(message, &state_clone).await;
                            if response_tx.send(response).await.is_err() {
                                error!("Failed to send IPC response");
                            }
                        }
                        None => break,
                    }
                }
                _ = msg_handler_shutdown.changed() => {
                    info!("Message handler task received shutdown signal");
                    break;
                }
            }
        }
        info!("Message handler task stopped");
    });

    // Tray is handled by the client on macOS, not the daemon
    // This is because macOS requires tray to be created on the main thread
    #[cfg(target_os = "macos")]
    {
        info!("Tray is managed by wakem client on macOS");
    }

    info!("Server is running (press Ctrl+C for graceful shutdown)");

    // Wait for exit signal (Ctrl+C)
    tokio::signal::ctrl_c().await?;

    // Trigger graceful shutdown
    info!("Initiating graceful shutdown...");
    shutdown.shutdown().await;

    // Signal bridge threads to exit
    input_shutdown_flag_stored.store(true, Ordering::SeqCst);
    #[cfg(target_os = "windows")]
    window_shutdown_flag.store(true, Ordering::SeqCst);

    // Wait a short time for tasks to clean up
    tokio::time::sleep(tokio::time::Duration::from_millis(SHUTDOWN_WAIT_DELAY_MS)).await;

    // Wait for all std threads to complete (with timeout)
    info!(
        "Waiting for {} background threads to complete...",
        thread_handles.len()
    );
    for (index, handle) in thread_handles.into_iter().enumerate() {
        match handle.join() {
            Ok(_) => debug!("Thread {} completed successfully", index),
            Err(e) => error!("Thread {} panicked: {:?}", index, e),
        }
    }

    info!("Server shutdown complete");
    Ok(())
}

/// Handle window events (Windows only)
#[cfg(target_os = "windows")]
impl ServerState {
    async fn handle_window_event(&self, event: crate::platform::windows::WindowEvent) {
        // Check if auto-apply preset is enabled
        let auto_apply = {
            let config = self.config.read().await;
            config.window.auto_apply_preset
        };

        if !auto_apply {
            return;
        }

        match event {
            crate::platform::windows::WindowEvent::WindowActivated(hwnd_isize) => {
                // Delay applying preset to ensure window is fully created
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    WINDOW_PRESET_APPLY_DELAY_MS,
                ))
                .await;

                // Get preset manager first, then create HWND and apply preset
                // This avoids holding HWND across await points (HWND is not Send)
                let preset_manager = self.window_preset_manager.read().await;
                let hwnd = windows::Win32::Foundation::HWND(
                    hwnd_isize as *mut std::ffi::c_void,
                );
                match preset_manager.apply_preset_for_window(hwnd) {
                    Ok(true) => {
                        debug!("Auto-applied preset to window {:?}", hwnd);
                    }
                    Ok(false) => {
                        // No matching preset, this is normal
                    }
                    Err(e) => {
                        debug!("Failed to auto-apply preset: {}", e);
                    }
                }
            }
        }
    }
}

/// Handle IPC messages
async fn handle_message(message: Message, state: &ServerState) -> Message {
    match message {
        Message::SetConfig { config } => match state.load_config(*config).await {
            Ok(_) => Message::ConfigLoaded,
            Err(e) => Message::ConfigError {
                error: e.to_string(),
            },
        },
        Message::ReloadConfig => match state.reload_config_from_file().await {
            Ok(_) => Message::ConfigLoaded,
            Err(e) => Message::ConfigError {
                error: e.to_string(),
            },
        },
        Message::SaveConfig => match state.save_config_to_file().await {
            Ok(_) => Message::ConfigLoaded,
            Err(e) => Message::ConfigError {
                error: e.to_string(),
            },
        },
        Message::GetStatus => {
            let (active, loaded) = state.get_status().await;
            Message::StatusResponse {
                active,
                config_loaded: loaded,
            }
        }
        Message::SetActive { active } => {
            state.set_active(active).await;
            Message::StatusResponse {
                active,
                config_loaded: *state.config_loaded.read().await,
            }
        }
        Message::Ping => Message::Pong,
        // Macro-related messages
        Message::StartMacroRecording { name } => {
            match state.start_macro_recording(&name).await {
                Ok(_) => Message::Success,
                Err(e) => Message::Error {
                    message: format!("Failed to start recording: {}", e),
                },
            }
        }
        Message::StopMacroRecording => match state.stop_macro_recording().await {
            Ok(macro_def) => Message::MacroRecordingResult {
                name: macro_def.name,
                action_count: macro_def.steps.len(),
            },
            Err(e) => Message::Error {
                message: format!("Failed to stop recording: {}", e),
            },
        },
        Message::PlayMacro { name } => match state.play_macro(&name).await {
            Ok(_) => Message::Success,
            Err(e) => Message::Error {
                message: format!("Failed to play macro: {}", e),
            },
        },
        Message::GetMacros => {
            let macros = state.get_macros().await;
            Message::MacrosList { macros }
        }
        Message::DeleteMacro { name } => match state.delete_macro(&name).await {
            Ok(_) => Message::Success,
            Err(e) => Message::Error {
                message: format!("Failed to delete macro: {}", e),
            },
        },
        Message::BindMacro {
            macro_name,
            trigger,
        } => match state.bind_macro(&macro_name, &trigger).await {
            Ok(_) => Message::Success,
            Err(e) => Message::Error {
                message: format!("Failed to bind macro: {}", e),
            },
        },
        Message::RegisterMessageWindow { hwnd } => {
            // Pass isize directly to avoid Send issues with HWND
            state.set_message_window_hwnd(hwnd as isize).await;
            Message::Success
        }
        _ => Message::Error {
            message: "Unknown message".to_string(),
        },
    }
}

/// Handle tray command (macOS only)
/// This is called synchronously from the menu callback on the main thread
/// Following Windows design: handle command directly without async
#[cfg(target_os = "macos")]
fn handle_tray_command(cmd: TrayAppCommand, state: Arc<ServerState>) {
    use tokio::runtime::Handle;

    let handle = Handle::current();

    match cmd {
        TrayAppCommand::ToggleActive => {
            info!("Tray: Toggle active command received");
            handle.spawn(async move {
                let current = *state.active.read().await;
                let new_state = !current;
                state.set_active(new_state).await;
                info!("Tray: Toggled active state to {}", new_state);
            });
        }
        TrayAppCommand::ReloadConfig => {
            info!("Tray: Reload config command received");
            handle.spawn(async move {
                if let Err(e) = state.reload_config_from_file().await {
                    error!("Tray: Failed to reload config: {}", e);
                    let _ = state
                        .show_notification(
                            "wakem",
                            &format!("Failed to reload config: {}", e),
                        )
                        .await;
                } else {
                    info!("Tray: Config reloaded successfully");
                    let _ = state
                        .show_notification("wakem", "Config reloaded successfully")
                        .await;
                }
            });
        }
        TrayAppCommand::OpenConfigFolder => {
            info!("Tray: Open config folder command received");
            // Get config path synchronously using try_read
            let config_path = state.config.try_read().ok().and_then(|config| {
                crate::config::resolve_config_file_path(None, config.network.instance_id)
            });

            if let Some(path) = config_path {
                let folder = if path.is_file() {
                    path.parent().map(|p| p.to_path_buf()).unwrap_or(path)
                } else {
                    path
                };

                let launcher = Launcher::new();
                if let Err(e) = launcher.open(&folder.to_string_lossy()) {
                    error!("Tray: Failed to open config folder: {}", e);
                } else {
                    info!("Tray: Opened config folder: {:?}", folder);
                }
            } else {
                warn!("Tray: Config path not found");
            }
        }
        TrayAppCommand::Exit => {
            info!("Tray: Exit command received");
            // Note: Exit is handled by main shutdown mechanism
            // We could trigger shutdown here if needed
        }
    }
}
