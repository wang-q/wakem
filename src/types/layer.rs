use serde::{Deserialize, Serialize};

use crate::types::{Action, MappingRule, Trigger};

/// 层模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerMode {
    /// 按住激活，释放退出
    Hold,
    /// 切换模式（按一次进入，再按一次退出）
    Toggle,
}

impl Default for LayerMode {
    fn default() -> Self {
        LayerMode::Hold
    }
}

/// 层定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    /// 层名称
    pub name: String,
    /// 激活键（扫描码）
    pub activation_key: u16,
    /// 激活键虚拟键码
    pub activation_vk: u16,
    /// 层模式
    pub mode: LayerMode,
    /// 层内映射规则
    pub mappings: Vec<MappingRule>,
}

impl Layer {
    pub fn new(
        name: impl Into<String>,
        activation_scan: u16,
        activation_vk: u16,
    ) -> Self {
        Self {
            name: name.into(),
            activation_key: activation_scan,
            activation_vk,
            mode: LayerMode::Hold,
            mappings: Vec::new(),
        }
    }

    pub fn with_mode(mut self, mode: LayerMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn add_mapping(&mut self, trigger: Trigger, action: Action) {
        self.mappings.push(MappingRule::new(trigger, action));
    }

    /// 检查是否是此层的激活键
    pub fn is_activation_key(&self, scan_code: u16, vk: u16) -> bool {
        self.activation_key == scan_code || self.activation_vk == vk
    }
}

/// 层栈（管理多个激活的层）
#[derive(Debug, Clone, Default)]
pub struct LayerStack {
    /// 基础层（最底层）
    base_layer: Vec<MappingRule>,
    /// 当前激活的层（按优先级排序）
    active_layers: Vec<Layer>,
    /// 当前按住激活的层
    hold_layers: Vec<String>,
}

impl LayerStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置基础层映射
    pub fn set_base_layer(&mut self, mappings: Vec<MappingRule>) {
        self.base_layer = mappings;
    }

    /// 激活一个层
    pub fn activate_layer(&mut self, layer: Layer) {
        // 如果层已经激活，先移除再添加（移到栈顶）
        self.active_layers.retain(|l| l.name != layer.name);
        self.active_layers.push(layer);
    }

    /// 停用指定层
    pub fn deactivate_layer(&mut self, name: &str) {
        self.active_layers.retain(|l| l.name != name);
        self.hold_layers.retain(|n| n != name);
    }

    /// 标记层为按住激活
    pub fn hold_layer(&mut self, name: &str) {
        if !self.hold_layers.contains(&name.to_string()) {
            self.hold_layers.push(name.to_string());
        }
    }

    /// 释放按住激活的层
    pub fn release_layer(&mut self, name: &str) {
        self.hold_layers.retain(|n| n != name);
        // 如果是 Hold 模式的层，释放时停用
        self.active_layers.retain(|l| {
            if l.name == name && l.mode == LayerMode::Hold {
                return false;
            }
            true
        });
    }

    /// 切换层的激活状态（用于 Toggle 模式）
    pub fn toggle_layer(&mut self, layer: Layer) {
        let is_active = self.active_layers.iter().any(|l| l.name == layer.name);
        if is_active {
            self.deactivate_layer(&layer.name);
        } else {
            self.activate_layer(layer);
        }
    }

    /// 获取所有当前可用的映射规则（按优先级：后激活的层优先）
    pub fn get_all_mappings(&self) -> Vec<MappingRule> {
        let mut result = Vec::new();

        // 先添加基础层
        result.extend(self.base_layer.clone());

        // 再添加激活的层（后面的层会覆盖前面的）
        for layer in &self.active_layers {
            result.extend(layer.mappings.clone());
        }

        result
    }

    /// 检查层是否激活
    #[allow(dead_code)]
    pub fn is_layer_active(&self, name: &str) -> bool {
        self.active_layers.iter().any(|l| l.name == name)
    }

    /// 获取当前激活的层列表
    #[allow(dead_code)]
    pub fn get_active_layers(&self) -> &[Layer] {
        &self.active_layers
    }

    /// 清空所有激活的层
    #[allow(dead_code)]
    pub fn clear_active_layers(&mut self) {
        self.active_layers.clear();
        self.hold_layers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_stack() {
        let mut stack = LayerStack::new();

        // 创建测试层
        let mut layer1 = Layer::new("navigate", 0x3A, 0x14);
        layer1.add_mapping(
            Trigger::key(0x23, 0x48),                  // H
            Action::key(KeyAction::click(0x4B, 0x25)), // Left
        );

        // 激活层
        stack.activate_layer(layer1.clone());
        assert!(stack.is_layer_active("navigate"));

        // 获取映射
        let mappings = stack.get_all_mappings();
        assert_eq!(mappings.len(), 1);

        // 停用层
        stack.deactivate_layer("navigate");
        assert!(!stack.is_layer_active("navigate"));
    }

    #[test]
    fn test_layer_toggle() {
        let mut stack = LayerStack::new();
        let layer = Layer::new("test", 0x3A, 0x14).with_mode(LayerMode::Toggle);

        // 切换激活
        stack.toggle_layer(layer.clone());
        assert!(stack.is_layer_active("test"));

        // 再次切换，停用
        stack.toggle_layer(layer);
        assert!(!stack.is_layer_active("test"));
    }
}
