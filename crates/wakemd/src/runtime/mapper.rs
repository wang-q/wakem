use std::collections::HashMap;
use tracing::{debug, trace};
use wakem_common::types::{Action, InputEvent, KeyAction, KeyEvent, MappingRule, Trigger};

/// 键位映射引擎
pub struct KeyMapper {
    /// 基础映射表：扫描码 -> 动作
    mappings: HashMap<u16, Action>,
    /// 完整的映射规则列表
    rules: Vec<MappingRule>,
    /// 是否启用
    enabled: bool,
}

impl KeyMapper {
    /// 创建新的映射引擎
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
            rules: Vec::new(),
            enabled: true,
        }
    }

    /// 从配置加载映射规则
    pub fn load_rules(&mut self, rules: Vec<MappingRule>) {
        self.rules = rules;
        self.rebuild_mappings();
        debug!("Loaded {} mapping rules", self.rules.len());
    }

    /// 添加单条映射规则
    pub fn add_rule(&mut self, rule: MappingRule) {
        self.rules.push(rule);
        self.rebuild_mappings();
    }

    /// 清除所有映射
    pub fn clear(&mut self) {
        self.rules.clear();
        self.mappings.clear();
    }

    /// 启用/禁用映射
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        debug!("KeyMapper enabled: {}", enabled);
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 处理输入事件，返回要执行的动作
    pub fn process_event(&self, event: &InputEvent) -> Option<Action> {
        if !self.enabled {
            return None;
        }

        match event {
            InputEvent::Key(key_event) => {
                self.process_key_event(key_event)
            }
            InputEvent::Mouse(_) => {
                // 鼠标事件处理（TODO）
                None
            }
        }
    }

    /// 处理键盘事件
    fn process_key_event(&self, event: &KeyEvent) -> Option<Action> {
        trace!(
            "Processing key event: scan_code={:04X}, vk={:04X}, state={:?}",
            event.scan_code, event.virtual_key, event.state
        );

        // 查找映射
        if let Some(action) = self.mappings.get(&event.scan_code) {
            // 根据按键状态调整动作
            let adjusted_action = match (action, &event.state) {
                (Action::Key(KeyAction::Click { scan_code, virtual_key }), _) => {
                    // 如果是点击动作，根据实际按键状态调整
                    match event.state {
                        wakem_common::types::KeyState::Pressed => {
                            Some(Action::Key(KeyAction::Press {
                                scan_code: *scan_code,
                                virtual_key: *virtual_key,
                            }))
                        }
                        wakem_common::types::KeyState::Released => {
                            Some(Action::Key(KeyAction::Release {
                                scan_code: *scan_code,
                                virtual_key: *virtual_key,
                            }))
                        }
                    }
                }
                _ => Some(action.clone()),
            };

            if adjusted_action.is_some() {
                trace!("Mapping found: {:04X} -> {:?}", event.scan_code, action);
            }

            return adjusted_action;
        }

        None
    }

    /// 重建映射表
    fn rebuild_mappings(&mut self) {
        self.mappings.clear();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            // 提取简单按键映射
            if let Trigger::Key { scan_code, virtual_key, .. } = &rule.trigger {
                if let Some(sc) = scan_code {
                    self.mappings.insert(*sc, rule.action.clone());
                } else if let Some(vk) = virtual_key {
                    // 如果没有扫描码，使用虚拟键码作为备用
                    self.mappings.insert(*vk, rule.action.clone());
                }
            }
        }

        debug!("Rebuilt mappings: {} entries", self.mappings.len());
    }

    /// 添加简单的键位重映射
    pub fn add_simple_remap(&mut self, from_scan_code: u16, to_scan_code: u16, to_vk: u16) {
        let trigger = Trigger::key(from_scan_code, 0);
        let action = Action::key(KeyAction::click(to_scan_code, to_vk));
        
        self.add_rule(MappingRule::new(trigger, action));
    }
}

impl Default for KeyMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wakem_common::types::{KeyState, KeyEvent};

    #[test]
    fn test_key_mapper() {
        let mut mapper = KeyMapper::new();
        
        // 添加 CapsLock -> Backspace 映射
        mapper.add_simple_remap(0x3A, 0x0E, 0x08);
        
        // 测试按下 CapsLock
        let event = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
        let result = mapper.process_event(&InputEvent::Key(event));
        
        assert!(result.is_some());
        match result.unwrap() {
            Action::Key(KeyAction::Press { scan_code, virtual_key }) => {
                assert_eq!(scan_code, 0x0E);
                assert_eq!(virtual_key, 0x08);
            }
            _ => panic!("Expected Press action"),
        }
    }

    #[test]
    fn test_disabled_mapper() {
        let mut mapper = KeyMapper::new();
        mapper.add_simple_remap(0x3A, 0x0E, 0x08);
        mapper.set_enabled(false);
        
        let event = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
        let result = mapper.process_event(&InputEvent::Key(event));
        
        assert!(result.is_none());
    }
}
