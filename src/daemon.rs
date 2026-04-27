use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};
use zeroize::Zeroizing;

use crate::config::Config;
use crate::constants::{
    INPUT_BATCH_SIZE_LIMIT, INPUT_BATCH_TIMEOUT_MICROS, INPUT_CHANNEL_CAPACITY,
    IPC_CHANNEL_CAPACITY, SHUTDOWN_WAIT_DELAY_MS,
};
use crate::ipc::{IpcServer, Message};
use crate::platform::traits::{
    ContextProvider, InputDeviceConfig, InputDeviceTrait, LauncherTrait,
    NotificationService, OutputDeviceTrait, PlatformFactory, PlatformUtilities,
    WindowManagerTrait, WindowPresetManagerTrait,
};
use crate::runtime::macro_player::{MacroContext, MacroPlayer};
use crate::shutdown::ShutdownSignal;
use crate::types::{
    macros::MacroRecorder, Action, InputEvent, KeyState, Macro, ModifierState,
};

use crate::runtime::{KeyMapper, LayerManager};

/// Server state
///
/// Performance optimization notes:
/// - Use RwLock instead of Mutex (for read-heavy scenarios)
/// - Group related states to reduce lock count
/// - Use Arc to share config and rules to avoid repeated cloning
///
/// Combined config state to reduce lock count.
/// `loaded` flag is kept together with `config` since they are always
/// accessed together and should be consistent.
///
/// Note: `auth_key` is stored as a separate `Arc<RwLock<Zeroizing<String>>>`
/// in `ServerState` rather than here, because it must be shared with
/// `IpcServer` via `Arc` cloning. Embedding it in ConfigState would require
/// nested locking (RwLock<ConfigState> containing another RwLock), which
/// adds complexity without meaningful benefit.
#[derive(Default)]
struct ConfigState {
    config: Config,
    loaded: bool,
}

/// Lock ordering rules (to prevent deadlocks):
/// 1. hyper_key_map (read) -> active_hyper_keys (write) - in check_and_update_hyper_key
/// 2. hyper_key_map (read) -> active_hyper_keys (read) - in merge_virtual_modifiers
/// 3. config (read) - in process_wheel_enhancement
/// 4. layer_manager (write) - in process_input_event
/// 5. mapper (read) - in process_input_event
/// 6. output_device -> mapper (read) -> launcher - in play_macro
///
/// IMPORTANT: Never hold multiple locks simultaneously except as documented above.
/// Always acquire locks in the order specified to prevent deadlocks.
pub struct ServerState {
    config: Arc<RwLock<ConfigState>>,
    mapper: Arc<RwLock<KeyMapper>>,
    layer_manager: Arc<RwLock<LayerManager>>,
    output_device: Arc<Mutex<Box<dyn OutputDeviceTrait + Send + Sync>>>,
    launcher: Arc<Mutex<Box<dyn LauncherTrait + Send + Sync>>>,
    window_preset_manager: Arc<RwLock<Box<dyn WindowPresetManagerTrait>>>,
    active: Arc<AtomicBool>,
    macro_recorder: Arc<MacroRecorder>,
    notification_service: Arc<Mutex<Box<dyn NotificationService>>>,
    auth_key: Arc<RwLock<Zeroizing<String>>>,
    active_hyper_keys: Arc<RwLock<std::collections::HashMap<(u16, u16), ModifierState>>>,
    hyper_key_map: Arc<RwLock<std::collections::HashMap<(u16, u16), ModifierState>>>,
    shutdown_signal: ShutdownSignal,
}

impl ServerState {
    pub fn new(shutdown_signal: ShutdownSignal) -> Self {
        use crate::platform::CurrentPlatform;

        let mut mapper = KeyMapper::new();
        mapper.set_window_manager(Box::new(CurrentPlatform::create_window_manager()));

        // Create services for KeyMapper (using parking_lot for sync access)
        let notification_service_for_mapper = Arc::new(parking_lot::Mutex::new(
            Box::new(CurrentPlatform::create_notification_service())
                as Box<dyn NotificationService>,
        ));
        let window_preset_manager_for_mapper = Arc::new(parking_lot::RwLock::new(
            Box::new(CurrentPlatform::create_window_preset_manager())
                as Box<dyn WindowPresetManagerTrait>,
        ));

        mapper.set_notification_service(notification_service_for_mapper);
        mapper.set_window_preset_manager(window_preset_manager_for_mapper);

        // Create separate services for ServerState async operations (using tokio::sync)
        let window_preset_manager = CurrentPlatform::create_window_preset_manager();
        let notification_service = CurrentPlatform::create_notification_service();

        Self {
            config: Arc::new(RwLock::new(ConfigState::default())),
            mapper: Arc::new(RwLock::new(mapper)),
            layer_manager: Arc::new(RwLock::new(LayerManager::new())),
            output_device: Arc::new(Mutex::new(Box::new(
                CurrentPlatform::create_output_device(),
            ))),
            launcher: Arc::new(Mutex::new(Box::new(CurrentPlatform::create_launcher()))),
            window_preset_manager: Arc::new(RwLock::new(Box::new(
                window_preset_manager,
            ))),
            active: Arc::new(AtomicBool::new(true)),
            macro_recorder: Arc::new(MacroRecorder::new()),
            notification_service: Arc::new(Mutex::new(Box::new(notification_service))),
            auth_key: Arc::new(RwLock::new(Zeroizing::new(String::new()))),
            active_hyper_keys: Arc::new(RwLock::new(std::collections::HashMap::new())),
            hyper_key_map: Arc::new(RwLock::new(std::collections::HashMap::new())),
            shutdown_signal,
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
        // Lock acquisition order (must be consistent to prevent deadlocks):
        // 1. auth_key       2. mapper         3. hyper_key_map
        // 4. preset_manager 5. layer_manager   6. config (includes loaded flag)
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
            *key = Zeroizing::new(config.network.auth_key.clone().unwrap_or_default());
        }

        // 2. Update base mapping rules and context rules (merged into one write lock)
        // Compute all_rules once and share between mapper and layer_manager
        let all_rules = config.get_all_rules();
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

        // 4. Update layer manager (reuse all_rules computed above)
        {
            let mut layer_manager = self.layer_manager.write().await;

            // Load base mappings (reuse all_rules, avoid recomputing)
            layer_manager.set_base_mappings(all_rules);

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

        // 5. Update config and mark as loaded (single lock acquisition)
        {
            let mut cfg = self.config.write().await;
            cfg.config = config;
            cfg.loaded = true;
        }

        info!("Configuration loaded successfully");
        Ok(())
    }

    /// Reload configuration from file
    pub async fn reload_config_from_file(&self) -> Result<()> {
        use crate::config::resolve_config_file_path;

        info!("Reloading configuration from file...");

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

        let new_config = match Config::from_file(&config_path) {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load config: {}", e);
                return Err(anyhow::anyhow!("Failed to load config: {}", e));
            }
        };

        self.load_config(new_config).await?;

        crate::config::invalidate_config_path_cache(_instance_id);

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
            let id = config.config.network.instance_id;
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
        if !self.active.load(Ordering::Acquire) {
            return;
        }

        if event.is_injected() {
            return;
        }

        if self.macro_recorder.is_recording().await {
            self.macro_recorder.record_event(&event).await;
        }

        if let Some(action) = self.process_wheel_event(&event).await {
            if let Err(e) = self.execute_action(action).await {
                error!(error = %e, "Failed to execute wheel action");
            }
            return;
        }

        let _is_hyper_key = self.check_and_update_hyper_key(&event).await;

        let event = self.merge_virtual_modifiers(event).await;

        if let InputEvent::Key(ref key_event) = event {
            debug!(
                scan_code = key_event.scan_code,
                virtual_key = key_event.virtual_key,
                state = ?key_event.state,
                modifiers = ?key_event.modifiers,
                "Processing key event"
            );
        }

        if self.should_filter_key_release(&event).await {
            return;
        }

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

        let action = {
            let mapper = self.mapper.read().await;
            let context: Option<crate::platform::traits::WindowContext> =
                get_current_window_context();
            mapper.process_event_with_context(&event, context.as_ref())
        };

        debug!(action = ?action, "Mapper processing result");

        if let Some(action) = action {
            if let Err(e) = self.execute_action(action).await {
                error!("Failed to execute action: {}", e);
            }
        } else {
            debug!("No action found for event");
        }
    }

    /// Process wheel enhancement for mouse wheel events.
    /// Returns Some(action) if a wheel event was enhanced, None otherwise.
    async fn process_wheel_event(&self, event: &InputEvent) -> Option<Action> {
        if let InputEvent::Mouse(mouse_event) = event {
            if let crate::types::MouseEventType::Wheel(delta) = mouse_event.event_type {
                debug!(wheel_delta = delta, "Processing wheel enhancement");
                return self.process_wheel_enhancement(delta).await;
            }
        }
        None
    }

    /// Check if a key release event should be filtered out.
    ///
    /// Hyper key releases must pass through (to clear virtual_modifiers),
    /// but other releases are blocked to prevent double-triggering of
    /// shortcut actions.
    async fn should_filter_key_release(&self, event: &InputEvent) -> bool {
        if let InputEvent::Key(key_event) = event {
            if key_event.state == KeyState::Released {
                let hyper_map = self.hyper_key_map.read().await;
                if !hyper_map.contains_key(&(key_event.scan_code, key_event.virtual_key)) {
                    debug!("Filtered non-hyper key release event");
                    return true;
                }
            }
        }
        false
    }

    /// Check if this is a hyper key and update virtual modifiers
    /// A hyper key is any key remapped to a modifier combination (e.g., CapsLock -> Ctrl+Alt+Meta)
    /// Returns true if this is a hyper key event
    ///
    /// Lock ordering: hyper_key_map (read) -> active_hyper_keys (write)
    ///
    /// Locks are acquired and released in separate scopes to avoid holding
    /// any lock across an await point. The hyper_key_map read lock is
    /// dropped before active_hyper_keys write lock is acquired.
    async fn check_and_update_hyper_key(&self, event: &InputEvent) -> bool {
        if let InputEvent::Key(key_event) = event {
            let modifiers = {
                let hyper_key_map = self.hyper_key_map.read().await;
                hyper_key_map
                    .get(&(key_event.scan_code, key_event.virtual_key))
                    .copied()
            };

            if let Some(modifiers) = modifiers {
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

    /// Merge virtual modifiers from active hyper keys into the event
    ///
    /// Locks are acquired and released in separate scopes to avoid holding
    /// any lock across an await point. The hyper_key_map read lock is
    /// dropped before active_hyper_keys read lock is acquired.
    async fn merge_virtual_modifiers(&self, mut event: InputEvent) -> InputEvent {
        if let InputEvent::Key(ref mut key_event) = event {
            let is_hyper_key = {
                let hyper_key_map = self.hyper_key_map.read().await;
                hyper_key_map.contains_key(&(key_event.scan_code, key_event.virtual_key))
            };

            if is_hyper_key {
                return event;
            }

            let merged = {
                let active = self.active_hyper_keys.read().await;
                if active.is_empty() {
                    None
                } else {
                    let mut merged = ModifierState::new();
                    for mods in active.values() {
                        merged.shift |= mods.shift;
                        merged.ctrl |= mods.ctrl;
                        merged.alt |= mods.alt;
                        merged.meta |= mods.meta;
                    }
                    Some(merged)
                }
            };

            if let Some(merged) = merged {
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
    ///
    /// Note: Nested sequences are flattened iteratively to prevent stack overflow
    async fn execute_action_sequence_optimized(&self, actions: &[Action]) -> Result<()> {
        let flattened = flatten_action_sequence(actions)?;
        self.execute_flattened_sequence(&flattened).await
    }

    /// Execute a flattened sequence of actions
    async fn execute_flattened_sequence(&self, actions: &[Action]) -> Result<()> {
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

                // Sequence actions should have been flattened
                Sequence(_) => {
                    // This should not happen if flattening worked correctly
                    error!("Unexpected nested sequence found during execution");
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

    /// Save macro to config
    ///
    /// In-memory state is updated under the write lock, then the lock is
    /// released before performing file I/O. The file save acquires a read
    /// lock so that other read operations are not blocked during the
    /// (potentially slow) disk write.
    async fn save_macro(&self, macro_def: &Macro) -> Result<()> {
        let config_path = {
            let mut config_state = self.config.write().await;
            config_state
                .config
                .macros
                .insert(macro_def.name.clone(), macro_def.steps.clone());
            crate::config::resolve_config_file_path(
                None,
                config_state.config.network.instance_id,
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
    ///
    /// Lock acquisition strategy: only acquire locks for resources actually
    /// needed by the macro's action types. This reduces lock contention for
    /// the common case (macros containing only Key/Mouse/Delay actions).
    ///
    /// - Key/Mouse/Delay/None actions: output_device only
    /// - Window actions: output_device + mapper (read)
    /// - Launch actions: output_device + launcher
    /// - Window + Launch: output_device + mapper (read) + launcher
    ///
    /// Lock ordering: output_device -> mapper (read) -> launcher
    /// This order must be consistent across all code paths that acquire
    /// multiple device locks simultaneously to prevent deadlocks.
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

        drop(config);

        let needs_wm = action_needs_window_manager(&macro_def.steps);
        let needs_launcher = action_needs_launcher(&macro_def.steps);

        let output_device = self.output_device.lock().await;
        let output_ref: &(dyn OutputDeviceTrait + Send + Sync) = output_device.as_ref();

        if !needs_wm && !needs_launcher {
            MacroPlayer::play_macro(output_ref, &macro_def, None, None).await?;
        } else {
            let window_manager_guard = if needs_wm {
                Some(self.mapper.read().await)
            } else {
                None
            };
            let launcher_guard = if needs_launcher {
                Some(self.launcher.lock().await)
            } else {
                None
            };

            let wm_ref = window_manager_guard.as_ref().and_then(|wm| {
                wm.window_manager
                    .as_ref()
                    .map(|w| w.as_ref() as &(dyn WindowManagerTrait + Send + Sync))
            });
            let launcher_ref = launcher_guard
                .as_ref()
                .map(|l| l.as_ref() as &(dyn LauncherTrait + Send + Sync));

            let context = MacroContext {
                window_manager: wm_ref,
                launcher: launcher_ref,
            };
            MacroPlayer::play_macro(output_ref, &macro_def, None, Some(context)).await?;
        }

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
    ///
    /// In-memory state is updated under the write lock, then the lock is
    /// released before performing file I/O (see `save_macro` for rationale).
    pub async fn delete_macro(&self, name: &str) -> Result<()> {
        let config_path = {
            let mut config = self.config.write().await;
            if config.config.macros.shift_remove(name).is_none() {
                return Err(anyhow::anyhow!("Macro '{}' not found", name));
            }
            config.config.macro_bindings.retain(|_, v| v != name);
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

        info!("Macro '{}' deleted", name);
        Ok(())
    }

    /// Bind macro to trigger key
    ///
    /// In-memory state is updated under the write lock, then the lock is
    /// released before performing file I/O (see `save_macro` for rationale).
    pub async fn bind_macro(&self, macro_name: &str, trigger: &str) -> Result<()> {
        let config_path = {
            let mut config = self.config.write().await;

            if !config.config.macros.contains_key(macro_name) {
                return Err(anyhow::anyhow!("Macro '{}' not found", macro_name));
            }

            config
                .config
                .macro_bindings
                .insert(trigger.to_string(), macro_name.to_string());
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

        info!("Macro '{}' bound to '{}'", macro_name, trigger);
        Ok(())
    }

    /// Initialize platform-specific services (e.g., notification service)
    pub async fn init_notification_service(
        &self,
        ctx: &crate::platform::traits::NotificationInitContext,
    ) {
        let service = self.notification_service.lock().await;
        service.initialize(ctx);
        info!("Platform services initialized");
    }

    /// Show notification using platform-abstracted notification service
    pub async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        let service = self.notification_service.lock().await;
        service.show(title, message)
    }

    /// Trigger graceful shutdown
    pub fn shutdown(&self) {
        info!("Triggering graceful shutdown...");
        self.shutdown_signal.shutdown();
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

/// Check if any macro step requires window manager (recursively checks Sequence actions)
fn action_needs_window_manager(steps: &[crate::types::MacroStep]) -> bool {
    steps.iter().any(|s| action_contains_window(&s.action))
}

/// Check if any macro step requires launcher (recursively checks Sequence actions)
fn action_needs_launcher(steps: &[crate::types::MacroStep]) -> bool {
    steps.iter().any(|s| action_contains_launch(&s.action))
}

/// Flatten nested action sequences recursively.
///
/// Preserves the correct order of actions when Sequence and non-Sequence
/// elements are interleaved (e.g., [A, Sequence([B, C]), D] -> [A, B, C, D]).
fn flatten_action_sequence(actions: &[Action]) -> Result<Vec<Action>> {
    const MAX_SEQUENCE_DEPTH: usize = 10;
    const MAX_TOTAL_ACTIONS: usize = 1000;

    let mut result = Vec::new();
    let mut total = 0usize;
    flatten_recursive(actions, 0, &mut result, &mut total, MAX_SEQUENCE_DEPTH, MAX_TOTAL_ACTIONS)?;
    Ok(result)
}

fn flatten_recursive(
    actions: &[Action],
    depth: usize,
    result: &mut Vec<Action>,
    total: &mut usize,
    max_depth: usize,
    max_total: usize,
) -> Result<()> {
    if depth > max_depth {
        return Err(anyhow::anyhow!(
            "Action sequence nesting exceeds maximum depth of {}",
            max_depth
        ));
    }

    for action in actions {
        match action {
            Action::Sequence(nested) => {
                flatten_recursive(nested, depth + 1, result, total, max_depth, max_total)?;
            }
            other => {
                *total += 1;
                if *total > max_total {
                    return Err(anyhow::anyhow!(
                        "Action sequence exceeds maximum total actions of {}",
                        max_total
                    ));
                }
                result.push(other.clone());
            }
        }
    }

    Ok(())
}

fn action_contains_window(action: &Action) -> bool {
    match action {
        Action::Window(_) => true,
        Action::Sequence(actions) => actions.iter().any(action_contains_window),
        _ => false,
    }
}

fn action_contains_launch(action: &Action) -> bool {
    match action {
        Action::Launch(_) => true,
        Action::Sequence(actions) => actions.iter().any(action_contains_launch),
        _ => false,
    }
}

/// Get current modifier key state using platform abstraction layer
fn get_current_modifier_state() -> ModifierState {
    <crate::platform::CurrentPlatform as PlatformUtilities>::get_modifier_state()
}

fn get_current_window_context() -> Option<crate::platform::traits::WindowContext> {
    <crate::platform::CurrentPlatform as ContextProvider>::get_current_context()
}

/// Initialize server state and load configuration
async fn initialize_server(
    instance_id: u32,
    preloaded_config: Option<Config>,
    config_path: Option<std::path::PathBuf>,
) -> Result<(Arc<ServerState>, Arc<ShutdownSignal>)> {
    info!("Starting wakemd server (instance {})...", instance_id);

    let shutdown = Arc::new(ShutdownSignal::new());
    let state = Arc::new(ServerState::new((*shutdown).clone()));

    // Set instance ID
    {
        let mut config = state.config.write().await;
        config.config.network.instance_id = instance_id;
    }

    load_configuration(&state, instance_id, preloaded_config, config_path).await?;

    Ok((state, shutdown))
}

/// Load configuration from various sources
async fn load_configuration(
    state: &Arc<ServerState>,
    instance_id: u32,
    preloaded_config: Option<Config>,
    config_path: Option<std::path::PathBuf>,
) -> Result<()> {
    if let Some(cfg) = preloaded_config {
        let mut config = state.config.write().await;
        config.config.network.instance_id = instance_id;
        drop(config);
        state.load_config(cfg).await?;
        info!("Configuration loaded from preloaded config");
    } else if let Some(path) = config_path {
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
    Ok(())
}

/// Setup IPC server with authentication
async fn setup_ipc_server(
    state: &Arc<ServerState>,
) -> Result<(IpcServer, mpsc::Receiver<(Message, mpsc::Sender<Message>)>)> {
    let (message_tx, message_rx) = mpsc::channel(IPC_CHANNEL_CAPACITY);
    let bind_address = {
        let mut config = state.config.write().await;
        let addr = config.config.network.get_bind_address();
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
    Ok((ipc_server, message_rx))
}

/// Setup input processing pipeline
fn setup_input_processing(
    state: &Arc<ServerState>,
    shutdown: &Arc<ShutdownSignal>,
) -> (Arc<AtomicBool>, std::thread::JoinHandle<()>) {
    let (input_tx, input_rx) =
        tokio::sync::mpsc::channel::<InputEvent>(INPUT_CHANNEL_CAPACITY);
    let input_shutdown_flag = Arc::new(AtomicBool::new(false));
    let raw_input_shutdown_flag = Arc::new(AtomicBool::new(false));

    let input_tx_bridge = input_tx.clone();
    let input_shutdown_flag_clone = input_shutdown_flag.clone();
    let raw_input_shutdown_flag_clone = raw_input_shutdown_flag.clone();

    let handle = std::thread::spawn(move || {
        run_input_bridge(
            input_tx_bridge,
            input_shutdown_flag_clone,
            raw_input_shutdown_flag_clone,
        );
    });

    // Start async input processing task
    let state_clone = state.clone();
    let input_shutdown = shutdown.subscribe();
    tokio::spawn(async move {
        run_input_processor(state_clone, input_rx, input_shutdown).await;
    });

    (input_shutdown_flag, handle)
}

/// Run input bridge thread (std thread)
fn run_input_bridge(
    tx: tokio::sync::mpsc::Sender<InputEvent>,
    shutdown_flag: Arc<AtomicBool>,
    raw_shutdown: Arc<AtomicBool>,
) {
    let (std_tx, std_rx) = std::sync::mpsc::channel::<InputEvent>();
    let raw_shutdown_for_bridge = raw_shutdown.clone();

    let raw_input_handle = std::thread::spawn(move || {
        match <crate::platform::CurrentPlatform as PlatformFactory>::create_input_device(
            InputDeviceConfig::default(),
            Some(std_tx),
        ) {
            Ok(mut device) => {
                if let Err(e) = device.register() {
                    error!("Failed to register Raw Input device: {}", e);
                    return;
                }
                info!("Raw Input device initialized and registered");
                while !raw_shutdown.load(Ordering::SeqCst) {
                    device.poll_event();
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                info!("Raw Input thread shutting down");
                device.stop();
            }
            Err(e) => {
                error!("Failed to create Raw Input device: {}", e);
            }
        }
    });

    loop {
        if shutdown_flag.load(Ordering::SeqCst) {
            raw_shutdown_for_bridge.store(true, Ordering::SeqCst);
            let _ = raw_input_handle.join();
            break;
        }
        match std_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(event) => {
                if tx.blocking_send(event).is_err() {
                    raw_shutdown_for_bridge.store(true, Ordering::SeqCst);
                    let _ = raw_input_handle.join();
                    break;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                raw_shutdown_for_bridge.store(true, Ordering::SeqCst);
                let _ = raw_input_handle.join();
                break;
            }
        }
    }
    info!("Input bridge thread shutdown complete");
}

/// Run async input processor
async fn run_input_processor(
    state: Arc<ServerState>,
    mut input_rx: tokio::sync::mpsc::Receiver<InputEvent>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    use tokio::time::{Duration, Instant};

    let batch_size_limit = INPUT_BATCH_SIZE_LIMIT;
    let batch_timeout_micros = INPUT_BATCH_TIMEOUT_MICROS;
    let mut event_batch = Vec::with_capacity(batch_size_limit);

    loop {
        let batch_start = Instant::now();

        loop {
            match input_rx.try_recv() {
                Ok(event) => {
                    event_batch.push(event);
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
                    if event_batch.is_empty() {
                        break;
                    }
                    if batch_start.elapsed()
                        >= Duration::from_micros(batch_timeout_micros)
                    {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    if event_batch.is_empty() {
                        return;
                    }
                    break;
                }
            }
        }

        if event_batch.is_empty() {
            tokio::select! {
                event = input_rx.recv() => {
                    match event {
                        Some(event) => {
                            state.process_input_event(event).await;
                        }
                        None => break,
                    }
                }
                _ = shutdown.changed() => {
                    info!("Input processing task received shutdown signal");
                    break;
                }
            }
        } else {
            let batch_len = event_batch.len();
            if batch_len > 1 {
                debug!(batch_size = batch_len, "Processing event batch");
            }

            for event in event_batch.drain(..) {
                if shutdown.has_changed().unwrap_or(false) {
                    info!("Input processing task received shutdown signal during batch");
                    return;
                }
                state.process_input_event(event).await;
            }
        }
    }
    info!("Input processing task stopped");
}

/// Setup window event processing
fn setup_window_event_processing(
    state: &Arc<ServerState>,
    shutdown: &Arc<ShutdownSignal>,
) -> (Arc<AtomicBool>, std::thread::JoinHandle<()>) {
    let window_shutdown_flag = Arc::new(AtomicBool::new(false));
    let window_shutdown_flag_clone = window_shutdown_flag.clone();

    let hook_shutdown_flag = Arc::new(AtomicBool::new(false));
    let hook_shutdown_flag_clone = hook_shutdown_flag.clone();

    let handle = std::thread::spawn(move || {
        run_window_event_bridge(window_shutdown_flag_clone, hook_shutdown_flag);
    });

    // Start async window event processor
    let state_clone = state.clone();
    let window_shutdown = shutdown.subscribe();
    tokio::spawn(async move {
        run_window_event_processor(state_clone, window_shutdown).await;
    });

    (hook_shutdown_flag_clone, handle)
}

/// Run window event bridge thread
///
/// TODO(medium-priority): Window event forwarding is not yet fully implemented.
/// Events from the hook are currently discarded. To complete this feature:
/// 1. Forward events from `std_rx` to an async channel (e.g., tokio mpsc)
/// 2. Receive events in `run_window_event_processor`
/// 3. Call `ServerState::handle_window_event` for each event
///
/// This is needed for auto-apply preset on window activation (see
/// `handle_window_event` and `WINDOW_PRESET_APPLY_DELAY_MS`).
fn run_window_event_bridge(
    shutdown_flag: Arc<AtomicBool>,
    hook_shutdown: Arc<AtomicBool>,
) {
    let (std_tx, std_rx) =
        std::sync::mpsc::channel::<crate::platform::traits::PlatformWindowEvent>();

    let hook_shutdown_inner = hook_shutdown.clone();
    let hook_handle = std::thread::spawn(move || {
        let mut hook = <crate::platform::CurrentPlatform as PlatformFactory>::create_window_event_hook(std_tx);
        if let Err(e) = hook.start_with_shutdown(hook_shutdown_inner) {
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

    loop {
        if shutdown_flag.load(Ordering::SeqCst) {
            break;
        }
        match std_rx.try_recv() {
            Ok(event) => {
                debug!(?event, "Window event received (not yet forwarded)");
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                break;
            }
        }
    }

    hook_shutdown.store(true, Ordering::SeqCst);
    let _ = hook_handle.join();
    info!("Window event bridge thread shutdown complete");
}

/// Run async window event processor
///
/// Note: `_state` parameter is currently unused but kept for future use
/// when window event handling is fully implemented.
async fn run_window_event_processor(
    _state: Arc<ServerState>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    // Window event processor - waits for window events and handles them
    // Currently simplified as the actual event channel setup is in setup_window_event_processing
    tokio::select! {
        _ = shutdown.changed() => {
            info!("Window event handling task received shutdown signal");
        }
    }
    info!("Window event handling task stopped");
}

/// Start IPC server task
fn spawn_ipc_server_task(ipc_server: IpcServer, shutdown: &Arc<ShutdownSignal>) {
    let ipc_shutdown = shutdown.subscribe();
    tokio::spawn(async move {
        run_ipc_server(ipc_server, ipc_shutdown).await;
    });
}

/// Run IPC server loop
async fn run_ipc_server(
    mut ipc_server: IpcServer,
    shutdown: tokio::sync::watch::Receiver<bool>,
) {
    if let Err(e) = ipc_server.run(shutdown).await {
        error!("IPC server error: {}", e);
    }
    info!("IPC server task stopped");
}

/// Start IPC message handler task
fn spawn_message_handler_task(
    state: &Arc<ServerState>,
    message_rx: mpsc::Receiver<(Message, mpsc::Sender<Message>)>,
    shutdown: &Arc<ShutdownSignal>,
) {
    let state_clone = state.clone();
    let msg_handler_shutdown = shutdown.subscribe();
    tokio::spawn(async move {
        run_message_handler(state_clone, message_rx, msg_handler_shutdown).await;
    });
}

/// Run IPC message handler
async fn run_message_handler(
    state: Arc<ServerState>,
    mut message_rx: mpsc::Receiver<(Message, mpsc::Sender<Message>)>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            msg = message_rx.recv() => {
                match msg {
                    Some((message, response_tx)) => {
                        let response = handle_message(message, &state).await;
                        if response_tx.send(response).await.is_err() {
                            error!("Failed to send IPC response");
                        }
                    }
                    None => break,
                }
            }
            _ = shutdown.changed() => {
                info!("Message handler task received shutdown signal");
                break;
            }
        }
    }
    info!("Message handler task stopped");
}

/// Wait for shutdown signal
async fn wait_for_shutdown(state: &Arc<ServerState>, shutdown: &Arc<ShutdownSignal>) {
    let mut state_shutdown_rx = state.subscribe_shutdown();

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Ctrl+C received, initiating graceful shutdown...");
        }
        _ = state_shutdown_rx.changed() => {
            info!("External shutdown request received, initiating graceful shutdown...");
        }
    }

    shutdown.shutdown();
}

/// Cleanup resources and wait for threads to complete
async fn cleanup_server(
    input_shutdown_flag: Arc<AtomicBool>,
    window_shutdown_flag: Arc<AtomicBool>,
    thread_handles: Vec<std::thread::JoinHandle<()>>,
) {
    input_shutdown_flag.store(true, Ordering::SeqCst);
    window_shutdown_flag.store(true, Ordering::SeqCst);

    tokio::time::sleep(tokio::time::Duration::from_millis(SHUTDOWN_WAIT_DELAY_MS)).await;

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
}

/// Run server with optional config path
///
/// This function orchestrates the server startup and shutdown process.
/// It has been refactored into smaller functions for better maintainability.
pub async fn run_server_with_config(
    instance_id: u32,
    preloaded_config: Option<Config>,
    config_path: Option<std::path::PathBuf>,
) -> Result<()> {
    // Initialize server
    let (state, shutdown) =
        initialize_server(instance_id, preloaded_config, config_path).await?;

    // Setup IPC server
    let (ipc_server, message_rx) = setup_ipc_server(&state).await?;

    // Setup input processing
    let (input_shutdown_flag, input_handle) = setup_input_processing(&state, &shutdown);

    // Setup window event processing
    let (window_shutdown_flag, window_handle) =
        setup_window_event_processing(&state, &shutdown);

    // Collect thread handles
    let thread_handles: Vec<std::thread::JoinHandle<()>> =
        vec![input_handle, window_handle];

    // Start IPC tasks
    spawn_ipc_server_task(ipc_server, &shutdown);
    spawn_message_handler_task(&state, message_rx, &shutdown);

    info!("Server is running (press Ctrl+C for graceful shutdown)");

    // Wait for shutdown signal
    wait_for_shutdown(&state, &shutdown).await;

    // Cleanup
    cleanup_server(input_shutdown_flag, window_shutdown_flag, thread_handles).await;

    Ok(())
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
        Message::InitializePlatform { native_handle } => {
            let ctx = crate::platform::traits::NotificationInitContext { native_handle };
            state.init_notification_service(&ctx).await;
            Message::Success
        }
        Message::Shutdown => {
            info!("Shutdown command received, initiating graceful shutdown...");
            state.shutdown();
            Message::Success
        }
        _ => Message::Error {
            message: "Unknown message".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{KeyAction, KeyEvent, MouseEventType};

    #[test]
    fn test_check_modifier_match_ctrl() {
        let mut mods = ModifierState::new();
        mods.ctrl = true;
        assert!(ServerState::check_modifier_match("ctrl", &mods));
        assert!(ServerState::check_modifier_match("Ctrl", &mods));
        assert!(ServerState::check_modifier_match("control", &mods));
        assert!(ServerState::check_modifier_match("Control", &mods));
        assert!(!ServerState::check_modifier_match("alt", &mods));
    }

    #[test]
    fn test_check_modifier_match_alt() {
        let mut mods = ModifierState::new();
        mods.alt = true;
        assert!(ServerState::check_modifier_match("alt", &mods));
        assert!(ServerState::check_modifier_match("Alt", &mods));
        assert!(!ServerState::check_modifier_match("ctrl", &mods));
    }

    #[test]
    fn test_check_modifier_match_shift() {
        let mut mods = ModifierState::new();
        mods.shift = true;
        assert!(ServerState::check_modifier_match("shift", &mods));
        assert!(ServerState::check_modifier_match("Shift", &mods));
        assert!(!ServerState::check_modifier_match("alt", &mods));
    }

    #[test]
    fn test_check_modifier_match_meta() {
        let mut mods = ModifierState::new();
        mods.meta = true;
        assert!(ServerState::check_modifier_match("win", &mods));
        assert!(ServerState::check_modifier_match("Win", &mods));
        assert!(ServerState::check_modifier_match("meta", &mods));
        assert!(ServerState::check_modifier_match("Meta", &mods));
        assert!(ServerState::check_modifier_match("command", &mods));
        assert!(ServerState::check_modifier_match("Command", &mods));
        assert!(!ServerState::check_modifier_match("ctrl", &mods));
    }

    #[test]
    fn test_check_modifier_match_right_variants() {
        let mut mods = ModifierState::new();
        mods.ctrl = true;
        mods.alt = true;
        mods.shift = true;
        assert!(ServerState::check_modifier_match("rightctrl", &mods));
        assert!(ServerState::check_modifier_match("rightalt", &mods));
        assert!(ServerState::check_modifier_match("rightshift", &mods));
    }

    #[test]
    fn test_check_modifier_match_unknown() {
        let mods = ModifierState::new();
        assert!(!ServerState::check_modifier_match("unknown", &mods));
        assert!(!ServerState::check_modifier_match("", &mods));
        assert!(!ServerState::check_modifier_match("super", &mods));
    }

    #[test]
    fn test_check_modifier_match_empty_modifiers() {
        let mods = ModifierState::new();
        assert!(!ServerState::check_modifier_match("ctrl", &mods));
        assert!(!ServerState::check_modifier_match("alt", &mods));
        assert!(!ServerState::check_modifier_match("shift", &mods));
        assert!(!ServerState::check_modifier_match("win", &mods));
    }

    #[test]
    fn test_flatten_action_sequence_simple() {
        let actions = vec![
            Action::Key(KeyAction::click(0x1E, 0x41)),
            Action::Delay { milliseconds: 100 },
            Action::None,
        ];
        let result = flatten_action_sequence(&actions).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_flatten_action_sequence_nested() {
        let inner = vec![
            Action::Key(KeyAction::click(0x1F, 0x42)),
            Action::Key(KeyAction::click(0x20, 0x43)),
        ];
        let actions = vec![
            Action::Key(KeyAction::click(0x1E, 0x41)),
            Action::Sequence(inner),
            Action::None,
        ];
        let result = flatten_action_sequence(&actions).unwrap();
        assert_eq!(result.len(), 4);
        assert!(matches!(&result[0], Action::Key(KeyAction::Click { virtual_key: 0x41, .. })));
        assert!(matches!(&result[1], Action::Key(KeyAction::Click { virtual_key: 0x42, .. })));
        assert!(matches!(&result[2], Action::Key(KeyAction::Click { virtual_key: 0x43, .. })));
        assert!(matches!(&result[3], Action::None));
    }

    #[test]
    fn test_flatten_action_sequence_order_preserved() {
        let inner = vec![
            Action::Key(KeyAction::click(0x1F, 0x42)),
            Action::Key(KeyAction::click(0x20, 0x43)),
        ];
        let actions = vec![
            Action::Key(KeyAction::click(0x1E, 0x41)),
            Action::Sequence(inner),
            Action::Key(KeyAction::click(0x21, 0x44)),
        ];
        let result = flatten_action_sequence(&actions).unwrap();
        assert_eq!(result.len(), 4);
        let vk_codes: Vec<u16> = result
            .iter()
            .filter_map(|a| match a {
                Action::Key(KeyAction::Click { virtual_key, .. }) => Some(*virtual_key),
                Action::Key(KeyAction::Press { virtual_key, .. }) => Some(*virtual_key),
                Action::Key(KeyAction::Release { virtual_key, .. }) => Some(*virtual_key),
                _ => None,
            })
            .collect();
        assert_eq!(vk_codes, vec![0x41, 0x42, 0x43, 0x44]);
    }

    #[test]
    fn test_flatten_action_sequence_deeply_nested() {
        let level3 = vec![Action::Key(KeyAction::click(0x1E, 0x41))];
        let level2 = vec![Action::Sequence(level3)];
        let level1 = vec![Action::Sequence(level2)];
        let actions = vec![Action::Sequence(level1)];
        let result = flatten_action_sequence(&actions).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_flatten_action_sequence_depth_exceeded() {
        let mut actions = vec![Action::Key(KeyAction::click(0x1E, 0x41))];
        for _ in 0..12 {
            let inner = actions.clone();
            actions = vec![Action::Sequence(inner)];
        }
        let result = flatten_action_sequence(&actions);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("maximum depth"));
    }

    #[test]
    fn test_flatten_action_sequence_total_actions_exceeded() {
        let actions: Vec<Action> = (0..1001)
            .map(|_| Action::Key(KeyAction::click(0x1E, 0x41)))
            .collect();
        let result = flatten_action_sequence(&actions);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("maximum total actions"));
    }

    #[test]
    fn test_flatten_action_sequence_empty() {
        let actions: Vec<Action> = vec![];
        let result = flatten_action_sequence(&actions).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_flatten_action_sequence_empty_nested() {
        let actions = vec![Action::Sequence(vec![]), Action::None];
        let result = flatten_action_sequence(&actions).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_shutdown_signal_propagation() {
        let shutdown = ShutdownSignal::new();
        let state = ServerState::new(shutdown.clone());

        let mut rx = state.subscribe_shutdown();
        assert!(!*rx.borrow_and_update());

        state.shutdown();

        tokio::select! {
            _ = rx.changed() => {
                assert!(*rx.borrow());
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                panic!("Shutdown signal not received within timeout");
            }
        }
    }

    #[tokio::test]
    async fn test_multiple_config_loads() {
        let state = ServerState::new(ShutdownSignal::new());

        let config1: Config = toml::from_str(
            r#"
[keyboard.remap]
CapsLock = "Backspace"
"#,
        )
        .unwrap();
        state.load_config(config1).await.unwrap();
        let (_, loaded) = state.get_status().await;
        assert!(loaded);

        let config2: Config = toml::from_str(
            r#"
[keyboard.remap]
A = "B"
"#,
        )
        .unwrap();
        state.load_config(config2).await.unwrap();
        let (_, loaded) = state.get_status().await;
        assert!(loaded);
    }

    #[tokio::test]
    async fn test_load_config_with_auth_key() {
        let state = ServerState::new(ShutdownSignal::new());

        let config: Config = toml::from_str(
            r#"
[network]
auth_key = "my-secret-key"
"#,
        )
        .unwrap();
        let result = state.load_config(config).await;
        assert!(result.is_ok());

        let auth_key = state.auth_key.read().await;
        assert_eq!(auth_key.as_str(), "my-secret-key");
    }

    #[tokio::test]
    async fn test_load_config_without_auth_key() {
        let state = ServerState::new(ShutdownSignal::new());

        let config = Config::default();
        let result = state.load_config(config).await;
        assert!(result.is_ok());

        let auth_key = state.auth_key.read().await;
        assert!(auth_key.is_empty());
    }

    #[tokio::test]
    async fn test_hyper_key_processing() {
        let state = ServerState::new(ShutdownSignal::new());

        let config: Config = toml::from_str(
            r#"
[keyboard.remap]
CapsLock = "Ctrl+Alt+Meta"
"#,
        )
        .unwrap();
        state.load_config(config).await.unwrap();

        let (caps_sc, caps_vk) = crate::config::parse_key("CapsLock").unwrap();

        let press_event =
            InputEvent::Key(KeyEvent::new(caps_sc, caps_vk, KeyState::Pressed));
        state.process_input_event(press_event).await;

        let release_event =
            InputEvent::Key(KeyEvent::new(caps_sc, caps_vk, KeyState::Released));
        state.process_input_event(release_event).await;
    }

    #[tokio::test]
    async fn test_wheel_enhancement_with_acceleration() {
        let state = ServerState::new(ShutdownSignal::new());

        let config: Config = toml::from_str(
            r#"
[mouse.wheel]
acceleration = true
acceleration_multiplier = 3
"#,
        )
        .unwrap();
        state.load_config(config).await.unwrap();

        let mouse_event =
            crate::types::MouseEvent::new(MouseEventType::Wheel(120), 0, 0);
        let event = InputEvent::Mouse(mouse_event);
        state.process_input_event(event).await;
    }

    #[tokio::test]
    async fn test_wheel_enhancement_horizontal_scroll() {
        let state = ServerState::new(ShutdownSignal::new());

        let config: Config = toml::from_str(
            r#"
[mouse.wheel.horizontal_scroll]
modifier = "Shift"
step = 3
"#,
        )
        .unwrap();
        state.load_config(config).await.unwrap();

        let mouse_event =
            crate::types::MouseEvent::new(MouseEventType::Wheel(120), 0, 0);
        let event = InputEvent::Mouse(mouse_event);
        state.process_input_event(event).await;
    }
}
