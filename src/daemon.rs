use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::constants::{
    INPUT_BATCH_SIZE_LIMIT, INPUT_BATCH_TIMEOUT_MICROS, INPUT_CHANNEL_CAPACITY,
    IPC_CHANNEL_CAPACITY, SHUTDOWN_WAIT_DELAY_MS, WINDOW_EVENT_CHANNEL_CAPACITY,
    WINDOW_PRESET_APPLY_DELAY_MS,
};
use crate::ipc::{IpcServer, Message};
use crate::platform::traits::{
    InputDeviceTrait, LauncherTrait, NotificationService, OutputDeviceTrait,
    PlatformFactory, WindowPresetManagerTrait,
};
use crate::runtime::macro_player::MacroPlayer;
use crate::shutdown::ShutdownSignal;
use crate::types::{
    macros::MacroRecorder, Action, InputEvent, KeyState, Macro, ModifierState,
};

use crate::runtime::{KeyMapper, LayerManager};

use crate::platform::CurrentPlatform;

/// Combined config state to reduce lock count
#[derive(Default)]
struct ConfigState {
    config: Config,
    loaded: bool,
}

/// Server state
///
/// Lock acquisition order (must be consistent to prevent deadlocks):
/// 1. auth_key            2. mapper              3. hyper_key_map
/// 4. window_preset_mgr   5. layer_manager       6. config
/// 7. output_device       8. launcher            9. notification_service
/// 10. pressed_keys       11. active_hyper_keys
///
/// IMPORTANT: Always acquire locks in ascending order. Never acquire an
/// earlier-numbered lock while holding a later-numbered one.
pub struct ServerState {
    config: Arc<RwLock<ConfigState>>,
    mapper: Arc<RwLock<KeyMapper>>,
    layer_manager: Arc<RwLock<LayerManager>>,
    output_device: Arc<Mutex<Box<dyn OutputDeviceTrait + Send + Sync>>>,
    launcher: Arc<Mutex<Box<dyn LauncherTrait + Send + Sync>>>,
    window_preset_manager: Arc<RwLock<Box<dyn WindowPresetManagerTrait>>>,
    notification_service: Arc<Mutex<Box<dyn NotificationService>>>,
    active: Arc<AtomicBool>,
    macro_recorder: Arc<MacroRecorder>,
    auth_key: Arc<RwLock<String>>,
    active_hyper_keys: Arc<RwLock<std::collections::HashMap<(u16, u16), ModifierState>>>,
    hyper_key_map: Arc<RwLock<std::collections::HashMap<(u16, u16), ModifierState>>>,
    pressed_keys: Arc<RwLock<std::collections::HashSet<(u16, u16)>>>,
    shutdown_signal: ShutdownSignal,
}

impl ServerState {
    pub fn new(shutdown_signal: ShutdownSignal) -> Self {
        let window_manager = CurrentPlatform::create_window_manager();
        let notification_service = CurrentPlatform::create_notification_service();
        let window_preset_manager = CurrentPlatform::create_window_preset_manager();
        let mapper = KeyMapper::with_window_manager(
            Box::new(window_manager),
            Some(Box::new(notification_service)),
            Some(Box::new(window_preset_manager)),
        );

        Self {
            config: Arc::new(RwLock::new(ConfigState::default())),
            mapper: Arc::new(RwLock::new(mapper)),
            layer_manager: Arc::new(RwLock::new(LayerManager::new())),
            output_device: Arc::new(Mutex::new(Box::new(
                CurrentPlatform::create_output_device(),
            ))),
            launcher: Arc::new(Mutex::new(Box::new(CurrentPlatform::create_launcher()))),
            window_preset_manager: Arc::new(RwLock::new(Box::new(
                CurrentPlatform::create_window_preset_manager(),
            ))),
            notification_service: Arc::new(Mutex::new(Box::new(
                CurrentPlatform::create_notification_service(),
            ))),
            active: Arc::new(AtomicBool::new(true)),
            macro_recorder: Arc::new(MacroRecorder::new()),
            auth_key: Arc::new(RwLock::new(String::new())),
            active_hyper_keys: Arc::new(RwLock::new(std::collections::HashMap::new())),
            hyper_key_map: Arc::new(RwLock::new(std::collections::HashMap::new())),
            pressed_keys: Arc::new(RwLock::new(std::collections::HashSet::new())),
            shutdown_signal,
        }
    }

    /// Load configuration
    ///
    /// Performance optimization: batch updates to reduce lock hold time
    #[tracing::instrument(skip(self, config), fields(
        rules_count,
        layers_count = config.keyboard.layers.len(),
        presets_count = config.window.presets.len(),
        context_mappings_count = config.keyboard.context_mappings.len(),
    ))]
    pub async fn load_config(&self, config: Config) -> Result<()> {
        // Lock acquisition order (must be consistent to prevent deadlocks):
        // 1. auth_key       2. mapper         3. hyper_key_map
        // 4. preset_manager 5. layer_manager   6. config
        // 7. config_loaded
        // IMPORTANT: Always acquire locks in this order. Never acquire an earlier
        // lock while holding a later one, as this could cause deadlocks.
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
        let all_rules = config.get_all_rules();
        tracing::Span::current().record("rules_count", all_rules.len());
        {
            let mut mapper = self.mapper.write().await;
            mapper.load_rules(all_rules.clone());
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

        // 3. Update window preset manager
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
            layer_manager.set_base_mappings(all_rules.clone());

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
            cfg.config = config;
        }

        // 6. Mark config as loaded
        {
            let mut cfg = self.config.write().await;
            cfg.loaded = true;
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
            let id = config.config.network.instance_id;
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
    ///
    /// If `path` is provided, saves to that path; otherwise saves to the default
    /// config file path based on instance_id.
    pub async fn save_config_to_file(
        &self,
        path: Option<&std::path::Path>,
    ) -> Result<()> {
        use crate::config::resolve_config_file_path;

        info!("Saving configuration to file...");

        // Determine config file path
        let config_path = if let Some(p) = path {
            p.to_path_buf()
        } else {
            // Get current instance ID and resolve default config file path
            let (_instance_id, resolved_path) = {
                let config = self.config.read().await;
                let id = config.config.network.instance_id;
                let path = resolve_config_file_path(None, id);
                (id, path)
            };

            match resolved_path {
                Some(path) => path,
                None => {
                    return Err(anyhow::anyhow!("Config file path not found"));
                }
            }
        };

        info!("Saving config to: {:?}", config_path);

        // Get current config and save
        let config = self.config.read().await;
        match config.config.save_to_file(&config_path) {
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
        if !self.active.load(Ordering::Acquire) {
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

        // Log key event details for debugging
        if let InputEvent::Key(ref key_event) = event {
            debug!(
                scan_code = key_event.scan_code,
                virtual_key = key_event.virtual_key,
                state = ?key_event.state,
                modifiers = ?key_event.modifiers,
                "Processing key event"
            );
        }

        // Filter out key release events for non-hyper keys
        // This replaces the old RI_KEY_BREAK filter in input.rs which dropped ALL key-up events.
        // We need hyper-key releases to pass through (to clear virtual_modifiers),
        // but must block other releases to prevent double-triggering of shortcut actions.
        if let InputEvent::Key(ref key_event) = event {
            if key_event.state == KeyState::Released {
                let hyper_map = self.hyper_key_map.read().await;
                if !hyper_map.contains_key(&(key_event.scan_code, key_event.virtual_key))
                {
                    debug!("Filtered non-hyper key release event");
                    // Remove from pressed_keys on release
                    self.pressed_keys
                        .write()
                        .await
                        .remove(&(key_event.scan_code, key_event.virtual_key));
                    return;
                }
            }
        }

        // Filter out key repeat events (Windows sends repeated WM_KEYDOWN while held).
        // Only the first press should trigger an action; subsequent repeats are suppressed.
        if let InputEvent::Key(ref key_event) = event {
            if key_event.state == KeyState::Pressed {
                let key_id = (key_event.scan_code, key_event.virtual_key);
                let mut pressed = self.pressed_keys.write().await;
                if !pressed.insert(key_id) {
                    debug!(
                        scan_code = key_event.scan_code,
                        virtual_key = key_event.virtual_key,
                        "Filtered key repeat event"
                    );
                    return;
                }
            }
        }

        // First try to process through layer manager (optimization: reduce write lock hold time)
        let (handled, action) = {
            let mut layer_manager = self.layer_manager.write().await;
            layer_manager.process_event(&event)
        };

        debug!(handled = handled, action = ?action, "Layer manager processing result");

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
            let context: Option<crate::platform::traits::WindowContext> =
                <CurrentPlatform as crate::platform::traits::ContextProvider>::get_current_context();
            mapper.process_event_with_context(&event, context.as_ref())
        };

        debug!(action = ?action, "Mapper processing result");

        // Execute action (outside lock to avoid long lock hold)
        if let Some(action) = action {
            if let Err(e) = self.execute_action(action).await {
                error!("Failed to execute action: {}", e);
            }
        } else {
            debug!("No action found for event");
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
                let mut active = self.active_hyper_keys.write().await;
                let key_id = (key_event.scan_code, key_event.virtual_key);
                match key_event.state {
                    crate::types::KeyState::Pressed => {
                        active.insert(key_id, modifiers);
                        debug!(
                            scan_code = key_event.scan_code,
                            vk = key_event.virtual_key,
                            ?modifiers,
                            active_count = active.len(),
                            "Hyper key pressed, virtual modifiers activated"
                        );
                    }
                    crate::types::KeyState::Released => {
                        active.remove(&key_id);
                        debug!(
                            scan_code = key_event.scan_code,
                            vk = key_event.virtual_key,
                            active_count = active.len(),
                            "Hyper key released, its modifier contributions removed"
                        );
                    }
                }
                return true;
            }
        }
        false
    }

    async fn merge_virtual_modifiers(&self, mut event: InputEvent) -> InputEvent {
        if let InputEvent::Key(ref mut key_event) = event {
            let hyper_key_map = self.hyper_key_map.read().await;
            if hyper_key_map.contains_key(&(key_event.scan_code, key_event.virtual_key))
            {
                drop(hyper_key_map);
                return event;
            }
            drop(hyper_key_map);

            let active = self.active_hyper_keys.read().await;
            if !active.is_empty() {
                let mut merged = ModifierState::new();
                for mods in active.values() {
                    merged.shift |= mods.shift;
                    merged.ctrl |= mods.ctrl;
                    merged.alt |= mods.alt;
                    merged.meta |= mods.meta;
                }
                key_event.modifiers.shift |= merged.shift;
                key_event.modifiers.ctrl |= merged.ctrl;
                key_event.modifiers.alt |= merged.alt;
                key_event.modifiers.meta |= merged.meta;
            }
        }
        event
    }

    /// Process wheel enhancement
    async fn process_wheel_enhancement(&self, delta: i32) -> Option<Action> {
        let config = self.config.read().await;
        let wheel_config = &config.config.mouse.wheel;

        // Get current modifier state
        let modifiers = get_current_modifier_state();

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
                mapper.execute_action(&Action::Window(window_action))?;
                info!("Window action executed successfully");
            }
            Action::Launch(launch_action) => {
                let launcher = self.launcher.lock().await;
                launcher.launch(&launch_action)?;
            }
            Action::Sequence(actions) => {
                self.execute_action_sequence_optimized(&actions).await?;
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
                Key(_) | Mouse(_) => {
                    let output = self.output_device.lock().await;

                    while i < actions.len() {
                        match &actions[i] {
                            Key(key_action) => {
                                output.send_key_action(key_action)?;
                            }
                            Mouse(mouse_action) => {
                                output.send_mouse_action(mouse_action)?;
                            }
                            _ => break,
                        }
                        i += 1;
                    }
                }

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
        self.active.store(active, Ordering::Release);
        info!("Server active state: {}", active);
    }

    /// Get status
    pub async fn get_status(&self) -> (bool, bool) {
        (
            self.active.load(Ordering::Acquire),
            self.config.read().await.loaded,
        )
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

    /// Check if recording macro
    pub async fn is_recording_macro(&self) -> bool {
        self.macro_recorder.is_recording().await
    }

    /// Save macro to config
    async fn save_macro(&self, macro_def: &Macro) -> Result<()> {
        let config_path = {
            let mut config = self.config.write().await;
            config
                .config
                .macros
                .insert(macro_def.name.clone(), macro_def.steps.clone());
            crate::config::resolve_config_file_path(
                None,
                config.config.network.instance_id,
            )
        };

        if let Some(config_path) = config_path {
            let config = self.config.read().await;
            if let Err(e) = config.config.save_to_file(&config_path) {
                warn!("Failed to save config to file: {}", e);
            }
        }

        info!("Macro '{}' saved to config", macro_def.name);
        Ok(())
    }

    /// Play macro
    pub async fn play_macro(&self, name: &str) -> Result<()> {
        let config = self.config.read().await;
        let steps = config
            .config
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
        MacroPlayer::play_macro(&**output_device, &macro_def, None).await?;

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
        config.config.macros.keys().cloned().collect()
    }

    /// Delete macro
    pub async fn delete_macro(&self, name: &str) -> Result<()> {
        let mut config = self.config.write().await;
        if config.config.macros.remove(name).is_none() {
            return Err(anyhow::anyhow!("Macro '{}' not found", name));
        }

        // Also remove bindings
        config.config.macro_bindings.retain(|_, v| v != name);

        // Save to file (best effort - don't fail if file operations fail)
        if let Some(config_path) = crate::config::resolve_config_file_path(
            None,
            config.config.network.instance_id,
        ) {
            if let Err(e) = config.config.save_to_file(&config_path) {
                warn!("Failed to save config to file: {}", e);
            }
        }

        info!("Macro '{}' deleted", name);
        Ok(())
    }

    /// Bind macro to trigger key
    pub async fn bind_macro(&self, macro_name: &str, trigger: &str) -> Result<()> {
        let mut config = self.config.write().await;

        // Check if macro exists
        if !config.config.macros.contains_key(macro_name) {
            return Err(anyhow::anyhow!("Macro '{}' not found", macro_name));
        }

        // Add binding
        config
            .config
            .macro_bindings
            .insert(trigger.to_string(), macro_name.to_string());

        // Save to file (best effort - don't fail if file operations fail)
        if let Some(config_path) = crate::config::resolve_config_file_path(
            None,
            config.config.network.instance_id,
        ) {
            if let Err(e) = config.config.save_to_file(&config_path) {
                warn!("Failed to save config to file: {}", e);
            }
        }

        info!("Macro '{}' bound to '{}'", macro_name, trigger);
        Ok(())
    }

    /// Set message window handle
    /// Takes isize instead of HWND because HWND is not Send and cannot be used across await points
    pub async fn set_message_window_hwnd(&self, hwnd_value: isize) {
        let service = self.notification_service.lock().await;
        service.set_message_window_handle(hwnd_value);
        info!("Message window handle registered: {}", hwnd_value);
    }

    /// Show notification via platform notification service
    pub async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        let service = self.notification_service.lock().await;
        match service.show(title, message) {
            Ok(()) => Ok(()),
            Err(e) => {
                warn!("Failed to show notification: {}", e);
                Ok(())
            }
        }
    }

    /// Trigger graceful shutdown
    pub async fn shutdown(&self) {
        info!("Triggering graceful shutdown...");
        self.shutdown_signal.shutdown().await;
    }

    /// Subscribe to shutdown signal (for external listeners)
    pub fn subscribe_shutdown(&self) -> tokio::sync::watch::Receiver<bool> {
        self.shutdown_signal.subscribe()
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new(ShutdownSignal::new())
    }
}

#[cfg(test)]
impl ServerState {
    /// Check if a key is currently in the pressed_keys set (test-only accessor)
    pub async fn is_key_pressed(&self, scan_code: u16, virtual_key: u16) -> bool {
        self.pressed_keys
            .read()
            .await
            .contains(&(scan_code, virtual_key))
    }

    /// Get the number of currently pressed keys (test-only accessor)
    pub async fn pressed_key_count(&self) -> usize {
        self.pressed_keys.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::types::{InputEvent, KeyEvent, KeyState};

    #[tokio::test]
    async fn test_key_repeat_first_press_accepted() {
        let state = ServerState::new(ShutdownSignal::new());
        let _ = state.load_config(Config::default()).await;

        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        state.process_input_event(event).await;

        assert!(
            state.is_key_pressed(0x1E, 0x41).await,
            "First press should add key to pressed_keys"
        );
        assert_eq!(state.pressed_key_count().await, 1);
    }

    #[tokio::test]
    async fn test_key_repeat_filtered() {
        let state = ServerState::new(ShutdownSignal::new());
        let _ = state.load_config(Config::default()).await;

        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));

        state.process_input_event(event.clone()).await;
        assert!(state.is_key_pressed(0x1E, 0x41).await);

        state.process_input_event(event.clone()).await;
        state.process_input_event(event.clone()).await;
        state.process_input_event(event.clone()).await;

        assert_eq!(
            state.pressed_key_count().await,
            1,
            "Repeated presses should not add duplicate entries"
        );
    }

    #[tokio::test]
    async fn test_key_release_clears_pressed_state() {
        let state = ServerState::new(ShutdownSignal::new());
        let _ = state.load_config(Config::default()).await;

        let press = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        state.process_input_event(press).await;
        assert!(state.is_key_pressed(0x1E, 0x41).await);

        let release = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Released));
        state.process_input_event(release).await;

        assert!(
            !state.is_key_pressed(0x1E, 0x41).await,
            "Key release should remove key from pressed_keys"
        );
        assert_eq!(state.pressed_key_count().await, 0);
    }

    #[tokio::test]
    async fn test_key_repress_after_release() {
        let state = ServerState::new(ShutdownSignal::new());
        let _ = state.load_config(Config::default()).await;

        let press = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        let release = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Released));

        state.process_input_event(press.clone()).await;
        assert!(state.is_key_pressed(0x1E, 0x41).await);

        state.process_input_event(release).await;
        assert!(!state.is_key_pressed(0x1E, 0x41).await);

        state.process_input_event(press).await;
        assert!(
            state.is_key_pressed(0x1E, 0x41).await,
            "Key should be tracked again after release+repress"
        );
    }

    #[tokio::test]
    async fn test_different_keys_tracked_independently() {
        let state = ServerState::new(ShutdownSignal::new());
        let _ = state.load_config(Config::default()).await;

        let press_a = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        let press_b = InputEvent::Key(KeyEvent::new(0x30, 0x42, KeyState::Pressed));

        state.process_input_event(press_a.clone()).await;
        state.process_input_event(press_b.clone()).await;

        assert!(state.is_key_pressed(0x1E, 0x41).await);
        assert!(state.is_key_pressed(0x30, 0x42).await);
        assert_eq!(state.pressed_key_count().await, 2);

        state.process_input_event(press_a).await;
        assert_eq!(
            state.pressed_key_count().await,
            2,
            "Repeat of A should not add new entry"
        );
    }

    #[tokio::test]
    async fn test_key_repeat_full_cycle() {
        let state = ServerState::new(ShutdownSignal::new());
        let _ = state.load_config(Config::default()).await;

        let press = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        let release = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Released));

        state.process_input_event(press.clone()).await;
        assert!(
            state.is_key_pressed(0x1E, 0x41).await,
            "Step 1: first press"
        );

        state.process_input_event(press.clone()).await;
        assert!(
            state.is_key_pressed(0x1E, 0x41).await,
            "Step 2: repeat (filtered)"
        );
        assert_eq!(state.pressed_key_count().await, 1);

        state.process_input_event(press.clone()).await;
        assert_eq!(
            state.pressed_key_count().await,
            1,
            "Step 3: more repeats (filtered)"
        );

        state.process_input_event(release).await;
        assert!(!state.is_key_pressed(0x1E, 0x41).await, "Step 4: release");

        state.process_input_event(press).await;
        assert!(
            state.is_key_pressed(0x1E, 0x41).await,
            "Step 5: repress after release"
        );
    }
}

/// Get current modifier key state
fn get_current_modifier_state() -> ModifierState {
    <CurrentPlatform as crate::platform::traits::PlatformUtilities>::get_modifier_state()
}

/// Run server with optional config path
///
/// Improvement: integrated graceful shutdown mechanism, supports safe exit of all background tasks
pub async fn run_server_with_config(
    instance_id: u32,
    preloaded_config: Option<Config>,
    config_path: Option<std::path::PathBuf>,
) -> Result<()> {
    info!("Starting wakemd server (instance {})...", instance_id);

    // Create graceful shutdown signal (shared between ServerState and run_server)
    let shutdown = Arc::new(ShutdownSignal::new());
    let shutdown_for_tasks = shutdown.subscribe();

    let state = Arc::new(ServerState::new((*shutdown).clone()));

    // Set instance ID
    {
        let mut config = state.config.write().await;
        config.config.network.instance_id = instance_id;
    }

    // Load configuration on startup (prefer preloaded config to avoid re-parsing)
    if let Some(cfg) = preloaded_config {
        let mut config = state.config.write().await;
        config.config.network.instance_id = instance_id;
        drop(config);
        state.load_config(cfg).await?;
        info!("Configuration loaded from preloaded config");
    } else if let Some(path) = config_path {
        // Try to load from explicit config path
        info!("Loading config from: {:?}", path);
        match Config::from_file(&path) {
            Ok(cfg) => {
                let mut config = state.config.write().await;
                config.config.network.instance_id = instance_id;
                drop(config);
                state.load_config(cfg).await?;
                info!("Configuration loaded successfully from {:?}", path);
            }
            Err(e) => {
                warn!(
                    "Failed to load config from {:?}: {}. Using default config.",
                    path, e
                );
            }
        }
    } else if let Err(e) = state.reload_config_from_file().await {
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
        let addr = config.config.network.get_bind_address();
        // Ensure auth key exists (security requirement)
        config.config.network.ensure_auth_key();
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

        let raw_input_handle = std::thread::spawn(move || {
            match <CurrentPlatform as PlatformFactory>::create_input_device(
                crate::platform::traits::InputDeviceConfig::default(),
                Some(std_tx),
            ) {
                Ok(mut device) => {
                    if let Err(e) = device.register() {
                        error!("Failed to register input device: {}", e);
                        return;
                    }
                    info!("Input device initialized and registered");
                    while !raw_input_shutdown.load(Ordering::SeqCst) {
                        if let Err(e) = device.run_once() {
                            error!("Input device error: {}", e);
                            break;
                        }
                    }
                    info!("Input device thread shutting down");
                }
                Err(e) => {
                    error!("Failed to create input device: {}", e);
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

    // Create shutdown flag for window event bridge thread
    let window_shutdown_flag = Arc::new(AtomicBool::new(false));
    let window_shutdown_flag_clone = window_shutdown_flag.clone();

    // Start window event listener (for auto-applying presets)
    {
        let mut window_event_rx = {
            let (tx, rx) = tokio::sync::mpsc::channel::<
                crate::platform::traits::PlatformWindowEvent,
            >(WINDOW_EVENT_CHANNEL_CAPACITY);

            let hook_shutdown_flag = Arc::new(AtomicBool::new(false));
            let hook_shutdown_flag_clone = hook_shutdown_flag.clone();
            let shutdown_flag = window_shutdown_flag_clone;

            let window_bridge_handle = std::thread::spawn(move || {
                let (std_tx, std_rx) = std::sync::mpsc::channel::<
                    crate::platform::traits::PlatformWindowEvent,
                >();

                let hook_shutdown_flag_inner = hook_shutdown_flag.clone();
                let hook_handle = std::thread::spawn(move || {
                    let mut hook =
                        <CurrentPlatform as PlatformFactory>::create_window_event_hook(
                            std_tx,
                        );
                    if let Err(e) = hook.start_with_shutdown(hook_shutdown_flag_inner) {
                        error!("Failed to start window event hook: {}", e);
                    } else {
                        info!("Window event hook started");
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

    info!("Server is running (press Ctrl+C for graceful shutdown)");

    // Subscribe to state shutdown signal for external shutdown requests
    let mut state_shutdown_rx = state.subscribe_shutdown();

    // Wait for exit signal (Ctrl+C) or external shutdown request
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Ctrl+C received, initiating graceful shutdown...");
        }
        _ = state_shutdown_rx.changed() => {
            info!("External shutdown request received, initiating graceful shutdown...");
        }
    }

    // Trigger graceful shutdown
    shutdown.shutdown().await;

    // Signal bridge threads to exit
    input_shutdown_flag_stored.store(true, Ordering::SeqCst);
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

/// Handle window events
impl ServerState {
    async fn handle_window_event(
        &self,
        event: crate::platform::traits::PlatformWindowEvent,
    ) {
        let auto_apply = {
            let config = self.config.read().await;
            config.config.window.auto_apply_preset
        };

        if !auto_apply {
            return;
        }

        let crate::platform::traits::PlatformWindowEvent::WindowActivated {
            process_name,
            window_title,
            window_id,
        } = event;
        debug!(
            "Window activated: process='{}', title='{}'",
            process_name, window_title
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(
            WINDOW_PRESET_APPLY_DELAY_MS,
        ))
        .await;

        let preset_manager = self.window_preset_manager.read().await;
        match preset_manager.apply_preset_for_window_by_id(window_id) {
            Ok(true) => {
                debug!("Auto-applied preset to window {}", window_id);
            }
            Ok(false) => {}
            Err(e) => {
                debug!("Failed to auto-apply preset: {}", e);
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
        Message::SaveConfig => match state.save_config_to_file(None).await {
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
            Message::Success
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
        Message::Shutdown => {
            info!("Shutdown command received, initiating graceful shutdown...");
            state.shutdown().await;
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
                let current = state.active.load(Ordering::Acquire);
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
                crate::config::resolve_config_file_path(
                    None,
                    config.config.network.instance_id,
                )
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
