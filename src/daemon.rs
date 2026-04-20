use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info};

use crate::config::Config;
use crate::ipc::{IpcServer, Message};
use crate::runtime::macro_player::MacroPlayer;
use crate::shutdown::ShutdownSignal;
use crate::types::{macros::MacroRecorder, Action, InputEvent, Macro, ModifierState};

use crate::platform::windows::{
    Launcher, LegacyOutputDevice as OutputDevice,
    LegacyRawInputDevice as RawInputDevice, WindowManager, WindowPresetManager,
};
use crate::runtime::{KeyMapper, LayerManager};
use windows::Win32::Foundation::HWND;

/// 服务端状态
///
/// 性能优化说明：
/// - 使用 RwLock 替代 Mutex（适用于读多写少场景）
/// - 将相关状态分组以减少锁数量
/// - 配置和规则使用 Arc 共享避免重复克隆
pub struct ServerState {
    /// 当前配置（读多写少）
    config: Arc<RwLock<Config>>,
    /// 键位映射引擎（读多写少：每次事件都读取，仅在配置变更时写入）
    mapper: Arc<RwLock<KeyMapper>>,
    /// 层管理器（读多写少）
    layer_manager: Arc<RwLock<LayerManager>>,
    /// 输出设备（写多：每个动作都需要写入）
    output_device: Arc<Mutex<OutputDevice>>,
    /// 程序启动器（写多：启动程序时需要互斥）
    launcher: Arc<Mutex<Launcher>>,
    /// 窗口预设管理器（读写均衡）
    window_preset_manager: Arc<RwLock<WindowPresetManager>>,
    /// 是否启用映射（频繁读取，极少写入）
    active: Arc<RwLock<bool>>,
    /// 配置是否已加载
    config_loaded: Arc<RwLock<bool>>,
    /// 宏录制器（内部已有同步机制）
    macro_recorder: Arc<MacroRecorder>,
    /// 消息窗口句柄（用于发送通知）
    message_window_hwnd: Arc<RwLock<Option<HWND>>>,
    /// 认证密钥（独立存储，支持动态更新）
    auth_key: Arc<RwLock<String>>,
}

impl ServerState {
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
        }
    }

    /// 加载配置
    ///
    /// 性能优化：批量更新减少锁持有时间
    #[tracing::instrument(skip(self, config), fields(
        rules_count = config.get_all_rules().len(),
        layers_count = config.keyboard.layers.len(),
        presets_count = config.window.presets.len(),
        context_mappings_count = config.keyboard.context_mappings.len(),
    ))]
    pub async fn load_config(&self, config: Config) -> Result<()> {
        // 1. 更新认证密钥（独立于配置存储）
        {
            let mut key = self.auth_key.write().await;
            *key = config.network.auth_key.clone().unwrap_or_default();
        }

        // 2. 更新基础映射规则和上下文规则（合并为一次写锁）
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

        // 3. 更新窗口预设管理器
        {
            let mut preset_manager = self.window_preset_manager.write().await;
            preset_manager.load_presets(config.window.presets.clone());
            debug!(
                presets_count = config.window.presets.len(),
                "Loaded window presets"
            );
        }

        // 4. 更新层管理器
        {
            let mut layer_manager = self.layer_manager.write().await;

            // 加载基础映射
            let base_rules = config.get_all_rules();
            layer_manager.set_base_mappings(base_rules);

            // 加载层配置
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

        // 5. 最后更新配置（确保所有组件已准备好）
        {
            let mut cfg = self.config.write().await;
            *cfg = config;
        }

        // 6. 标记配置已加载
        {
            let mut loaded = self.config_loaded.write().await;
            *loaded = true;
        }

        info!("Configuration loaded successfully");
        Ok(())
    }

    /// 从文件重新加载配置
    pub async fn reload_config_from_file(&self) -> Result<()> {
        use crate::config::resolve_config_file_path;

        info!("Reloading configuration from file...");

        // 获取当前实例ID和配置文件路径
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

        // 尝试加载新配置
        let new_config = match Config::from_file(&config_path) {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load config: {}", e);
                return Err(anyhow::anyhow!("Failed to load config: {}", e));
            }
        };

        // 应用新配置
        self.load_config(new_config).await?;

        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// 保存当前配置到文件
    pub async fn save_config_to_file(&self) -> Result<()> {
        use crate::config::resolve_config_file_path;

        info!("Saving configuration to file...");

        // 获取当前实例ID和配置文件路径
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

        // 获取当前配置并保存
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

    /// 处理输入事件
    ///
    /// 性能优化：
    /// - 使用 RwLock.read() 替代 Mutex.lock()（读多写少场景）
    /// - 减少锁持有时间，快速路径优先返回
    /// - 批量读取相关状态
    #[tracing::instrument(skip(self, event), fields(event_type = %event.event_type_name()))]
    pub async fn process_input_event(&self, event: InputEvent) {
        // 快速路径：检查是否启用（最轻量的锁）
        if !*self.active.read().await {
            return;
        }

        // 如果是注入的事件，忽略（避免循环）
        if event.is_injected() {
            return;
        }

        // 如果正在录制宏，记录事件
        if self.macro_recorder.is_recording().await {
            self.macro_recorder.record_event(&event).await;
        }

        // 处理滚轮增强
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

        // 先尝试通过层管理器处理（需要写锁因为 process_event 会修改状态）
        let (handled, action) = {
            let mut layer_manager = self.layer_manager.write().await;
            layer_manager.process_event(&event)
        };

        if handled {
            // 如果层管理器处理了事件（包括层激活键）
            if let Some(action) = action {
                if let Err(e) = self.execute_action(action).await {
                    error!("Failed to execute action: {}", e);
                }
            }
            return;
        }

        // 层管理器未处理，使用基础映射引擎（带上下文感知）- 使用读锁
        let action = {
            let mapper = self.mapper.read().await;
            // 获取当前窗口上下文
            let context = crate::platform::windows::WindowContext::get_current();
            mapper.process_event_with_context(&event, context.as_ref())
        };

        // 执行动作
        if let Some(action) = action {
            if let Err(e) = self.execute_action(action).await {
                error!("Failed to execute action: {}", e);
            }
        }
    }

    /// 处理滚轮增强
    async fn process_wheel_enhancement(&self, delta: i32) -> Option<Action> {
        let config = self.config.read().await;
        let wheel_config = &config.mouse.wheel;

        // 获取当前修饰键状态
        let modifiers = get_current_modifier_state();

        // 检查音量控制
        if let Some(volume_config) = &wheel_config.volume_control {
            if Self::check_modifier_match(&volume_config.modifier, &modifiers) {
                if delta > 0 {
                    return Some(Action::System(crate::types::SystemAction::VolumeUp));
                } else {
                    return Some(Action::System(crate::types::SystemAction::VolumeDown));
                }
            }
        }

        // 检查亮度控制
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

        // 检查水平滚动
        if let Some(hscroll_config) = &wheel_config.horizontal_scroll {
            if Self::check_modifier_match(&hscroll_config.modifier, &modifiers) {
                // 将垂直滚轮转换为水平滚轮
                return Some(Action::Mouse(crate::types::MouseAction::HWheel {
                    delta: delta * hscroll_config.step,
                }));
            }
        }

        // 检查滚轮加速
        if wheel_config.acceleration {
            // 简单的加速实现：根据滚动方向增加滚动距离
            let accelerated_delta = delta * wheel_config.acceleration_multiplier as i32;
            return Some(Action::Mouse(crate::types::MouseAction::Wheel {
                delta: accelerated_delta,
            }));
        }

        None
    }

    /// 检查修饰键是否匹配
    fn check_modifier_match(modifier_str: &str, modifiers: &ModifierState) -> bool {
        match modifier_str.to_lowercase().as_str() {
            "shift" => modifiers.shift,
            "ctrl" | "control" => modifiers.ctrl,
            "alt" => modifiers.alt,
            "win" | "meta" | "command" => modifiers.meta,
            "rightalt" => modifiers.alt,     // 简化处理
            "rightctrl" => modifiers.ctrl,   // 简化处理
            "rightshift" => modifiers.shift, // 简化处理
            _ => false,
        }
    }

    /// 执行动作
    async fn execute_action(&self, action: Action) -> Result<()> {
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
                let mut mapper = self.mapper.write().await;
                mapper.execute_action(&Action::Window(window_action))?;
            }
            Action::Launch(launch_action) => {
                let launcher = self.launcher.lock().await;
                launcher.launch(&launch_action)?;
            }
            Action::Sequence(actions) => {
                for a in actions {
                    Box::pin(self.execute_action(a)).await?;
                }
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

    /// 设置启用状态
    pub async fn set_active(&self, active: bool) {
        let mut a = self.active.write().await;
        *a = active;
        info!("Server active state: {}", active);
    }

    /// 获取状态
    pub async fn get_status(&self) -> (bool, bool) {
        (*self.active.read().await, *self.config_loaded.read().await)
    }

    /// 开始录制宏
    pub async fn start_macro_recording(&self, name: &str) -> Result<()> {
        self.macro_recorder.start_recording(name).await
    }

    /// 停止录制宏
    pub async fn stop_macro_recording(&self) -> Result<Macro> {
        let macro_def = self.macro_recorder.stop_recording().await?;
        self.save_macro(&macro_def).await?;

        // 显示录制完成通知
        let step_count = macro_def.steps.len();
        let _ = self
            .show_notification(
                "wakem - 宏录制",
                &format!(
                    "宏 '{}' 录制完成，包含 {} 个步骤",
                    macro_def.name, step_count
                ),
            )
            .await;

        Ok(macro_def)
    }

    /// 保存宏到配置
    async fn save_macro(&self, macro_def: &Macro) -> Result<()> {
        let mut config = self.config.write().await;
        config
            .macros
            .insert(macro_def.name.clone(), macro_def.steps.clone());

        // 保存到文件
        let config_path =
            crate::config::resolve_config_file_path(None, config.network.instance_id)
                .ok_or_else(|| anyhow::anyhow!("Config path not found"))?;
        config.save_to_file(&config_path)?;

        info!("Macro '{}' saved to config", macro_def.name);
        Ok(())
    }

    /// 执行宏
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

        drop(config); // 释放读锁

        let output_device = self.output_device.lock().await;
        MacroPlayer::play_macro(&output_device, &macro_def).await?;

        // 显示回放完成通知
        let _ = self
            .show_notification("wakem - 宏回放", &format!("宏 '{}' 回放完成", name))
            .await;

        Ok(())
    }

    /// 获取宏列表
    pub async fn get_macros(&self) -> Vec<String> {
        let config = self.config.read().await;
        config.macros.keys().cloned().collect()
    }

    /// 删除宏
    pub async fn delete_macro(&self, name: &str) -> Result<()> {
        let mut config = self.config.write().await;
        if config.macros.remove(name).is_none() {
            return Err(anyhow::anyhow!("Macro '{}' not found", name));
        }

        // 同时删除绑定
        config.macro_bindings.retain(|_, v| v != name);

        // 保存到文件
        let config_path =
            crate::config::resolve_config_file_path(None, config.network.instance_id)
                .ok_or_else(|| anyhow::anyhow!("Config path not found"))?;
        config.save_to_file(&config_path)?;

        info!("Macro '{}' deleted", name);
        Ok(())
    }

    /// 绑定宏到触发键
    pub async fn bind_macro(&self, macro_name: &str, trigger: &str) -> Result<()> {
        let mut config = self.config.write().await;

        // 检查宏是否存在
        if !config.macros.contains_key(macro_name) {
            return Err(anyhow::anyhow!("Macro '{}' not found", macro_name));
        }

        // 添加绑定
        config
            .macro_bindings
            .insert(trigger.to_string(), macro_name.to_string());

        // 保存到文件
        let config_path =
            crate::config::resolve_config_file_path(None, config.network.instance_id)
                .ok_or_else(|| anyhow::anyhow!("Config path not found"))?;
        config.save_to_file(&config_path)?;

        info!("Macro '{}' bound to '{}'", macro_name, trigger);
        Ok(())
    }

    /// 检查是否正在录制宏
    pub async fn is_recording_macro(&self) -> bool {
        self.macro_recorder.is_recording().await
    }

    /// 设置消息窗口句柄
    pub async fn set_message_window_hwnd(&self, hwnd: HWND) {
        let mut h = self.message_window_hwnd.write().await;
        *h = Some(hwnd);
        info!("Message window handle registered: {:?}", hwnd);
    }

    /// 获取当前认证密钥（用于 IPC 认证）
    pub async fn get_auth_key(&self) -> String {
        self.auth_key.read().await.clone()
    }

    /// 显示托盘通知
    pub async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        if let Some(hwnd) = *self.message_window_hwnd.read().await {
            // 使用托盘图标显示通知
            self.show_tray_notification(hwnd, title, message).await?;
        } else {
            debug!("Message window not registered, skipping notification");
        }
        Ok(())
    }

    /// 使用托盘图标显示通知（内部方法）
    async fn show_tray_notification(
        &self,
        hwnd: HWND,
        title: &str,
        message: &str,
    ) -> Result<()> {
        use windows::Win32::UI::Shell::{
            NIF_INFO, NIM_MODIFY, NOTIFYICONDATAW, NOTIFY_ICON_INFOTIP_FLAGS,
        };

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1, // 托盘图标 ID
            uFlags: NIF_INFO,
            ..Default::default()
        };

        // 转换标题和消息为宽字符串
        let title_wide: Vec<u16> =
            title.encode_utf16().chain(std::iter::once(0)).collect();
        let message_wide: Vec<u16> =
            message.encode_utf16().chain(std::iter::once(0)).collect();

        // 复制到结构体（限制长度）
        let title_len = title_wide.len().min(64);
        let message_len = message_wide.len().min(256);

        nid.szInfoTitle[..title_len].copy_from_slice(&title_wide[..title_len]);
        nid.szInfo[..message_len].copy_from_slice(&message_wide[..message_len]);

        // 设置通知类型（0 = 无图标）
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

/// 获取当前修饰键状态
#[cfg(target_os = "windows")]
fn get_current_modifier_state() -> ModifierState {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_MENU,
        VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_SHIFT,
    };

    let mut modifiers = ModifierState::default();

    unsafe {
        // 检查 Shift 键
        if GetAsyncKeyState(VK_SHIFT.0 as i32) < 0
            || GetAsyncKeyState(VK_LSHIFT.0 as i32) < 0
            || GetAsyncKeyState(VK_RSHIFT.0 as i32) < 0
        {
            modifiers.shift = true;
        }

        // 检查 Ctrl 键
        if GetAsyncKeyState(VK_CONTROL.0 as i32) < 0
            || GetAsyncKeyState(VK_LCONTROL.0 as i32) < 0
            || GetAsyncKeyState(VK_RCONTROL.0 as i32) < 0
        {
            modifiers.ctrl = true;
        }

        // 检查 Alt 键
        if GetAsyncKeyState(VK_MENU.0 as i32) < 0
            || GetAsyncKeyState(VK_LMENU.0 as i32) < 0
            || GetAsyncKeyState(VK_RMENU.0 as i32) < 0
        {
            modifiers.alt = true;
        }

        // 检查 Win 键 (VK_LWIN = 0x5B, VK_RWIN = 0x5C)
        if GetAsyncKeyState(0x5B) < 0 || GetAsyncKeyState(0x5C) < 0 {
            modifiers.meta = true;
        }
    }

    modifiers
}

#[cfg(not(target_os = "windows"))]
fn get_current_modifier_state() -> ModifierState {
    ModifierState::default()
}

/// 运行服务端
///
/// 改进：集成优雅关闭机制，支持安全退出所有后台任务
pub async fn run_server(instance_id: u32) -> Result<()> {
    info!("Starting wakemd server (instance {})...", instance_id);

    let state = Arc::new(ServerState::new());

    // 创建优雅关闭信号
    let shutdown = Arc::new(ShutdownSignal::new());
    let shutdown_for_tasks = shutdown.subscribe();

    // 设置实例ID
    {
        let mut config = state.config.write().await;
        config.network.instance_id = instance_id;
    }

    // 创建 IPC 服务端（使用动态认证密钥）
    let (message_tx, mut message_rx) = mpsc::channel(100);
    let bind_address = {
        let mut config = state.config.write().await;
        let addr = config.network.get_bind_address();
        // 确保存在认证密钥（安全要求）
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

    // 创建输入事件通道（使用 tokio::sync::mpsc 用于高效的异步处理）
    let (input_tx, mut input_rx) = tokio::sync::mpsc::channel::<InputEvent>(1000);

    // 启动 Raw Input 捕获（在单独线程中，通过 bridge 发送到 tokio channel）
    let input_tx_bridge = input_tx.clone();
    std::thread::spawn(move || {
        let (std_tx, std_rx) = std::sync::mpsc::channel::<InputEvent>();
        let tx_clone = input_tx_bridge;

        std::thread::spawn(move || match RawInputDevice::new(std_tx) {
            Ok(mut device) => {
                info!("Raw Input device initialized");
                if let Err(e) = device.run() {
                    error!("Raw Input error: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to create Raw Input device: {}", e);
            }
        });

        // Bridge: 从 std channel 接收并发送到 tokio channel
        while let Ok(event) = std_rx.recv() {
            if tx_clone.blocking_send(event).is_err() {
                break; // 通道关闭，退出
            }
        }
        info!("Input bridge thread shutdown complete");
    });

    // 启动输入处理任务（带关闭信号检查）
    let state_clone = state.clone();
    let mut input_shutdown = shutdown_for_tasks.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                event = input_rx.recv() => {
                    match event {
                        Some(event) => {
                            state_clone.process_input_event(event).await;
                        }
                        None => break, // 通道关闭
                    }
                }
                _ = input_shutdown.changed() => {
                    info!("Input processing task received shutdown signal");
                    break;
                }
            }
        }
        info!("Input processing task stopped");
    });

    // 启动窗口事件监听（用于自动应用预设）
    let mut window_event_rx = {
        let (tx, rx) =
            tokio::sync::mpsc::channel::<crate::platform::windows::WindowEvent>(100);

        std::thread::spawn(move || {
            let (std_tx, std_rx) =
                std::sync::mpsc::channel::<crate::platform::windows::WindowEvent>();

            std::thread::spawn(move || {
                let mut hook = crate::platform::windows::WindowEventHook::new(std_tx);
                if let Err(e) = hook.start() {
                    error!("Failed to start window event hook: {}", e);
                } else {
                    info!("Window event hook started");
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                }
            });

            // Bridge: 从 std channel 接收并发送到 tokio channel
            while let Ok(event) = std_rx.recv() {
                if tx.blocking_send(event).is_err() {
                    break;
                }
            }
            info!("Window event bridge thread shutdown complete");
        });

        rx
    };

    // 启动窗口事件处理任务（带关闭信号检查）
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

    // 启动 IPC 服务端主循环（带关闭信号检查）
    let mut ipc_shutdown = shutdown_for_tasks.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = ipc_server.run() => {
                    if let Err(e) = result {
                        error!("IPC server error: {}", e);
                        // 发生错误后等待一小段时间再重试
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

    // 处理 IPC 消息（带关闭信号检查）
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

    // 等待退出信号（Ctrl+C）
    tokio::signal::ctrl_c().await?;

    // 触发优雅关闭
    info!("Initiating graceful shutdown...");
    shutdown.shutdown().await;

    // 等待一小段时间让任务清理完成
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    info!("Server shutdown complete");
    Ok(())
}

/// 处理窗口事件
impl ServerState {
    async fn handle_window_event(&self, event: crate::platform::windows::WindowEvent) {
        // 检查是否启用了自动应用预设
        let auto_apply = {
            let config = self.config.read().await;
            config.window.auto_apply_preset
        };

        if !auto_apply {
            return;
        }

        match event {
            crate::platform::windows::WindowEvent::WindowCreated(hwnd)
            | crate::platform::windows::WindowEvent::WindowActivated(hwnd) => {
                // 延迟一点应用预设，确保窗口已完全创建
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                let preset_manager = self.window_preset_manager.read().await;
                match preset_manager.apply_preset_for_window(hwnd) {
                    Ok(true) => {
                        debug!("Auto-applied preset to window {:?}", hwnd);
                    }
                    Ok(false) => {
                        // 没有匹配的预设，这是正常的
                    }
                    Err(e) => {
                        debug!("Failed to auto-apply preset: {}", e);
                    }
                }
            }
        }
    }
}

/// 处理 IPC 消息
async fn handle_message(message: Message, state: &ServerState) -> Message {
    match message {
        Message::SetConfig { config } => match state.load_config(config).await {
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
        // 宏相关消息
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
            let hwnd = windows::Win32::Foundation::HWND(hwnd as isize);
            state.set_message_window_hwnd(hwnd).await;
            Message::Success
        }
        _ => Message::Error {
            message: "Unknown message".to_string(),
        },
    }
}
