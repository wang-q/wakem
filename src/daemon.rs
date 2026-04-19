use anyhow::Result;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info};

use crate::config::Config;
use crate::ipc::{Message, IpcServer};
use crate::types::{Action, InputEvent};

use crate::platform::windows::{OutputDevice, Launcher, WindowManager, RawInputDevice};
use crate::runtime::{KeyMapper, LayerManager};

/// 服务端状态
pub struct ServerState {
    /// 当前配置
    config: Arc<RwLock<Config>>,
    /// 键位映射引擎
    mapper: Arc<Mutex<KeyMapper>>,
    /// 层管理器
    layer_manager: Arc<Mutex<LayerManager>>,
    /// 输出设备
    output_device: Arc<Mutex<OutputDevice>>,
    /// 程序启动器
    launcher: Arc<Mutex<Launcher>>,
    /// 窗口管理器
    window_manager: Arc<Mutex<WindowManager>>,
    /// 是否启用映射
    active: Arc<RwLock<bool>>,
    /// 配置是否已加载
    config_loaded: Arc<RwLock<bool>>,
}

impl ServerState {
    pub fn new() -> Self {
        let window_manager = WindowManager::new();
        let mapper = KeyMapper::with_window_manager(window_manager);

        Self {
            config: Arc::new(RwLock::new(Config::default())),
            mapper: Arc::new(Mutex::new(mapper)),
            layer_manager: Arc::new(Mutex::new(LayerManager::new())),
            output_device: Arc::new(Mutex::new(OutputDevice::new())),
            launcher: Arc::new(Mutex::new(Launcher::new())),
            window_manager: Arc::new(Mutex::new(WindowManager::new())),
            active: Arc::new(RwLock::new(true)),
            config_loaded: Arc::new(RwLock::new(false)),
        }
    }

    /// 加载配置
    pub async fn load_config(&self, config: Config) -> Result<()> {
        // 更新配置
        {
            let mut cfg = self.config.write().await;
            *cfg = config.clone();
        }

        // 更新基础映射规则
        {
            let mut mapper = self.mapper.lock().await;
            let rules = config.get_all_rules();
            mapper.load_rules(rules);
        }

        // 更新层管理器
        {
            let mut layer_manager = self.layer_manager.lock().await;
            
            // 加载基础映射
            let base_rules = config.get_all_rules();
            layer_manager.set_base_mappings(base_rules);
            
            // 加载层配置
            for (name, layer_config) in &config.keyboard.layers {
                let mode = match layer_config.mode {
                    crate::config::LayerMode::Hold => crate::types::LayerMode::Hold,
                    crate::config::LayerMode::Toggle => crate::types::LayerMode::Toggle,
                };
                let mappings: Vec<(String, String)> = layer_config.mappings
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

        // 标记配置已加载
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

        // 获取当前配置文件路径
        let config_path = {
            let config = self.config.read().await;
            // 尝试从配置中获取路径，或使用默认路径
            resolve_config_file_path(None)
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

    /// 处理输入事件
    pub async fn process_input_event(&self, event: InputEvent) {
        // 检查是否启用
        if !*self.active.read().await {
            return;
        }

        // 如果是注入的事件，忽略（避免循环）
        if event.is_injected() {
            return;
        }

        // 先尝试通过层管理器处理
        let (handled, action) = {
            let mut layer_manager = self.layer_manager.lock().await;
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

        // 层管理器未处理，使用基础映射引擎
        let action = {
            let mapper = self.mapper.lock().await;
            mapper.process_event(&event)
        };

        // 执行动作
        if let Some(action) = action {
            if let Err(e) = self.execute_action(action).await {
                error!("Failed to execute action: {}", e);
            }
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
                let mapper = self.mapper.lock().await;
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
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// 运行服务端
pub async fn run_server() -> Result<()> {
    info!("Starting wakemd server...");

    let state = Arc::new(ServerState::new());
    
    // 创建 IPC 服务端
    let (message_tx, mut message_rx) = mpsc::channel(100);
    let mut ipc_server = IpcServer::new(message_tx);
    ipc_server.start().await?;

    info!("IPC server started");

    // 创建输入事件通道（使用 std::sync::mpsc 用于 Raw Input 线程）
    let (input_tx, input_rx) = std::sync::mpsc::channel::<InputEvent>();
    let input_rx = Arc::new(StdMutex::new(input_rx));
    
    // 启动 Raw Input 捕获（在单独线程中）
    let input_tx_clone = input_tx.clone();
    std::thread::spawn(move || {
        match RawInputDevice::new(input_tx_clone) {
            Ok(mut device) => {
                info!("Raw Input device initialized");
                if let Err(e) = device.run() {
                    error!("Raw Input error: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to create Raw Input device: {}", e);
            }
        }
    });

    // 启动输入处理任务（将 std 通道转换为 tokio 任务）
    let state_clone = state.clone();
    let input_rx_clone = input_rx.clone();
    tokio::spawn(async move {
        loop {
            // 在非阻塞模式下检查接收
            let event = {
                let rx = input_rx_clone.lock().unwrap();
                rx.try_recv().ok()
            };
            
            if let Some(event) = event {
                state_clone.process_input_event(event).await;
            } else {
                // 没有事件时短暂休眠
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        }
    });

    // 启动 IPC 连接处理
    let _state_clone = state.clone();
    tokio::spawn(async move {
        loop {
            match ipc_server.accept().await {
                Ok(mut connection) => {
                    debug!("New IPC connection accepted");
                    
                    // 处理连接
                    tokio::spawn(async move {
                        if let Err(e) = connection.handle().await {
                            error!("IPC connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept IPC connection: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    });

    // 处理 IPC 消息
    let state_clone = state.clone();
    tokio::spawn(async move {
        while let Some((message, response_tx)) = message_rx.recv().await {
            let response = handle_message(message, &state_clone).await;
            let _ = response_tx.send(response).await;
        }
    });

    info!("Server is running");

    // 等待退出信号
    tokio::signal::ctrl_c().await?;
    
    info!("Shutting down server...");
    Ok(())
}

/// 处理 IPC 消息
async fn handle_message(message: Message, state: &ServerState) -> Message {
    match message {
        Message::SetConfig { config } => {
            match state.load_config(config).await {
                Ok(_) => Message::ConfigLoaded,
                Err(e) => Message::ConfigError { error: e.to_string() },
            }
        }
        Message::ReloadConfig => {
            match state.reload_config_from_file().await {
                Ok(_) => Message::ConfigLoaded,
                Err(e) => Message::ConfigError { error: e.to_string() },
            }
        }
        Message::GetStatus => {
            let (active, loaded) = state.get_status().await;
            Message::StatusResponse { 
                active, 
                config_loaded: loaded 
            }
        }
        Message::SetActive { active } => {
            state.set_active(active).await;
            Message::StatusResponse { 
                active, 
                config_loaded: *state.config_loaded.read().await 
            }
        }
        Message::Ping => Message::Pong,
        _ => Message::Error { message: "Unknown message".to_string() },
    }
}
