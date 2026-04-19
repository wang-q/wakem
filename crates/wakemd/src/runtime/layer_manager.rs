use std::collections::HashMap;
use tracing::{debug, trace};
use wakem_common::types::{Layer, LayerMode, LayerStack, MappingRule, Trigger, Action, KeyAction, InputEvent, KeyEvent, KeyState};

/// 层管理器
pub struct LayerManager {
    /// 所有可用的层
    layers: HashMap<String, Layer>,
    /// 层栈（管理激活状态）
    stack: LayerStack,
    /// 基础层映射（从配置加载的简单重映射）
    base_mappings: Vec<MappingRule>,
}

impl LayerManager {
    pub fn new() -> Self {
        Self {
            layers: HashMap::new(),
            stack: LayerStack::new(),
            base_mappings: Vec::new(),
        }
    }

    /// 注册一个层
    pub fn register_layer(&mut self, layer: Layer) {
        debug!("Registering layer: {}", layer.name);
        self.layers.insert(layer.name.clone(), layer);
    }

    /// 设置基础层映射
    pub fn set_base_mappings(&mut self, mappings: Vec<MappingRule>) {
        self.base_mappings = mappings.clone();
        self.stack.set_base_layer(mappings);
    }

    /// 处理输入事件，检查是否是层激活键
    /// 返回 (是否处理了事件, 可选的动作)
    pub fn process_event(&mut self, event: &InputEvent) -> (bool, Option<Action>) {
        match event {
            InputEvent::Key(key_event) => {
                self.process_key_event(key_event)
            }
            _ => (false, None),
        }
    }

    /// 处理键盘事件
    fn process_key_event(&mut self, event: &KeyEvent) -> (bool, Option<Action>) {
        // 检查是否是某个层的激活键
        for layer in self.layers.values() {
            if layer.is_activation_key(event.scan_code, event.virtual_key) {
                match layer.mode {
                    LayerMode::Hold => {
                        match event.state {
                            KeyState::Pressed => {
                                trace!("Activating layer (Hold): {}", layer.name);
                                let layer = self.layers.get(&layer.name).cloned().unwrap();
                                self.stack.hold_layer(&layer.name);
                                self.stack.activate_layer(layer);
                            }
                            KeyState::Released => {
                                trace!("Deactivating layer (Hold): {}", layer.name);
                                self.stack.release_layer(&layer.name);
                            }
                        }
                        // 层激活键本身不传递
                        return (true, None);
                    }
                    LayerMode::Toggle => {
                        if event.state == KeyState::Pressed {
                            trace!("Toggling layer: {}", layer.name);
                            let layer = self.layers.get(&layer.name).cloned().unwrap();
                            self.stack.toggle_layer(layer);
                            // 切换键本身不传递
                            return (true, None);
                        }
                    }
                }
            }
        }

        // 如果不是激活键，在激活的层中查找映射
        let input_event = InputEvent::Key(event.clone());
        let mappings = self.stack.get_all_mappings();
        for rule in &mappings {
            if rule.trigger.matches(&input_event) {
                trace!("Found mapping in layer: {:?}", rule.action);
                return (true, Some(rule.action.clone()));
            }
        }

        (false, None)
    }

    /// 获取当前激活的层列表
    pub fn get_active_layers(&self) -> Vec<String> {
        self.stack.get_active_layers()
            .iter()
            .map(|l| l.name.clone())
            .collect()
    }

    /// 检查层是否激活
    pub fn is_layer_active(&self, name: &str) -> bool {
        self.stack.is_layer_active(name)
    }

    /// 停用所有层
    pub fn clear_layers(&mut self) {
        self.stack.clear_active_layers();
    }

    /// 从配置创建层
    pub fn create_layer_from_config(
        name: &str,
        activation_key: &str,
        mode: LayerMode,
        mappings: &[(String, String)],
    ) -> anyhow::Result<Layer> {
        use wakem_common::config::parse_key;

        let (scan, vk) = parse_key(activation_key)?;
        let mut layer = Layer::new(name, scan, vk).with_mode(mode);

        for (from, to) in mappings {
            let from_key = parse_key(from)?;
            let to_key = parse_key(to)?;
            
            layer.add_mapping(
                Trigger::key(from_key.0, from_key.1),
                Action::key(KeyAction::click(to_key.0, to_key.1)),
            );
        }

        Ok(layer)
    }
}

impl Default for LayerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_manager_hold() {
        let mut manager = LayerManager::new();
        
        // 创建导航层
        let layer = LayerManager::create_layer_from_config(
            "navigate",
            "CapsLock",
            LayerMode::Hold,
            &[("H".to_string(), "Left".to_string())],
        ).unwrap();
        
        manager.register_layer(layer);
        
        // 模拟按下 CapsLock
        let press = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
        let (handled, _) = manager.process_event(&InputEvent::Key(press));
        assert!(handled);
        assert!(manager.is_layer_active("navigate"));
        
        // 模拟释放 CapsLock
        let release = KeyEvent::new(0x3A, 0x14, KeyState::Released);
        let (handled, _) = manager.process_event(&InputEvent::Key(release));
        assert!(handled);
        assert!(!manager.is_layer_active("navigate"));
    }

    #[test]
    fn test_layer_mapping() {
        let mut manager = LayerManager::new();
        
        // 创建导航层
        let layer = LayerManager::create_layer_from_config(
            "navigate",
            "CapsLock",
            LayerMode::Hold,
            &[("H".to_string(), "Left".to_string())],
        ).unwrap();
        
        manager.register_layer(layer);
        
        // 先激活层
        let caps_press = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
        manager.process_event(&InputEvent::Key(caps_press));
        
        // 模拟按下 H，应该映射为 Left
        let h_press = KeyEvent::new(0x23, 0x48, KeyState::Pressed);
        let (handled, action) = manager.process_event(&InputEvent::Key(h_press));
        assert!(handled);
        assert!(action.is_some());
    }
}
