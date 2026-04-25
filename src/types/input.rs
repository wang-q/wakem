use super::{now, DeviceType, KeyState, ModifierState, Timestamp, VirtualKey};
use serde::{Deserialize, Serialize};

/// Keyboard event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    /// Scan code (hardware related)
    pub scan_code: u16,
    /// Virtual key code (Windows VK_*)
    pub virtual_key: u16,
    /// Key state
    pub state: KeyState,
    /// Modifier key state
    pub modifiers: ModifierState,
    /// Device type
    pub device_type: DeviceType,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Whether from physical device (not simulated input)
    pub is_injected: bool,
}

impl KeyEvent {
    pub fn new(scan_code: u16, virtual_key: u16, state: KeyState) -> Self {
        Self {
            scan_code,
            virtual_key,
            state,
            modifiers: ModifierState::default(),
            device_type: DeviceType::Keyboard,
            timestamp: now(),
            is_injected: false,
        }
    }

    /// Set modifier key state (for building events)
    #[allow(dead_code)]
    pub fn with_modifiers(mut self, modifiers: ModifierState) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Mark as injected event (for simulated input)
    #[allow(dead_code)]
    pub fn injected(mut self) -> Self {
        self.is_injected = true;
        self
    }

    /// Check if is modifier key
    pub fn is_modifier(&self) -> bool {
        VirtualKey::new(self.virtual_key).is_modifier()
    }

    /// Get modifier key identifier (if is modifier key)
    #[allow(dead_code)]
    pub fn modifier_identifier(&self) -> Option<&'static str> {
        VirtualKey::new(self.virtual_key).modifier_name()
    }
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1, // Side button 1
    X2, // Side button 2
}

/// Mouse event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseEvent {
    /// Event type
    pub event_type: MouseEventType,
    /// X coordinate (screen coordinates)
    pub x: i32,
    /// Y coordinate (screen coordinates)
    pub y: i32,
    /// Modifier key state
    pub modifiers: ModifierState,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Whether from physical device
    pub is_injected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseEventType {
    /// Mouse move
    Move,
    /// Button press
    ButtonDown(MouseButton),
    /// Button release
    ButtonUp(MouseButton),
    /// Scroll wheel (positive up, negative down)
    Wheel(i32),
    /// Horizontal scroll (positive right, negative left)
    HWheel(i32),
}

impl MouseEvent {
    pub fn new(event_type: MouseEventType, x: i32, y: i32) -> Self {
        Self {
            event_type,
            x,
            y,
            modifiers: ModifierState::default(),
            timestamp: now(),
            is_injected: false,
        }
    }

    /// Set modifier key state (for building events)
    #[allow(dead_code)]
    pub fn with_modifiers(mut self, modifiers: ModifierState) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Mark as injected event (for simulated input)
    #[allow(dead_code)]
    pub fn injected(mut self) -> Self {
        self.is_injected = true;
        self
    }

    /// Check if is button press event
    pub fn is_button_down(&self, button: MouseButton) -> bool {
        matches!(&self.event_type, MouseEventType::ButtonDown(b) if *b == button)
    }

    /// Check if is button release event
    #[allow(dead_code)]
    pub fn is_button_up(&self, button: MouseButton) -> bool {
        matches!(&self.event_type, MouseEventType::ButtonUp(b) if *b == button)
    }
}

/// Input event (keyboard or mouse)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

impl InputEvent {
    /// Get event timestamp
    pub fn timestamp(&self) -> Timestamp {
        match self {
            InputEvent::Key(e) => e.timestamp,
            InputEvent::Mouse(e) => e.timestamp,
        }
    }

    pub fn is_injected(&self) -> bool {
        match self {
            InputEvent::Key(e) => e.is_injected,
            InputEvent::Mouse(e) => e.is_injected,
        }
    }

    /// Get event type name (for logging)
    pub fn event_type_name(&self) -> &'static str {
        match self {
            InputEvent::Key(_) => "key",
            InputEvent::Mouse(_) => "mouse",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test KeyEvent creation
    #[test]
    fn test_key_event_creation() {
        let event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);

        assert_eq!(event.scan_code, 0x1E);
        assert_eq!(event.virtual_key, 0x41);
        assert!(matches!(event.state, KeyState::Pressed));
        assert_eq!(event.device_type, DeviceType::Keyboard);
        assert!(!event.is_injected);
    }

    /// Test KeyState enum
    #[test]
    fn test_key_state_enum() {
        let pressed = KeyState::Pressed;
        let released = KeyState::Released;

        assert!(matches!(pressed, KeyState::Pressed));
        assert!(matches!(released, KeyState::Released));
    }

    /// Test MouseEvent creation
    #[test]
    fn test_mouse_event_creation() {
        let event = MouseEvent::new(MouseEventType::Move, 100, 200);

        assert_eq!(event.x, 100);
        assert_eq!(event.y, 200);
        assert!(matches!(event.event_type, MouseEventType::Move));
        assert!(!event.is_injected);
    }

    /// Test MouseButton enum
    #[test]
    fn test_mouse_button_enum() {
        let left = MouseButton::Left;
        let right = MouseButton::Right;
        let middle = MouseButton::Middle;
        let x1 = MouseButton::X1;
        let x2 = MouseButton::X2;

        assert!(matches!(left, MouseButton::Left));
        assert!(matches!(right, MouseButton::Right));
        assert!(matches!(middle, MouseButton::Middle));
        assert!(matches!(x1, MouseButton::X1));
        assert!(matches!(x2, MouseButton::X2));
    }

    /// Test InputEvent variants
    #[test]
    fn test_input_event_variants() {
        let key_event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));

        let mouse_event =
            InputEvent::Mouse(MouseEvent::new(MouseEventType::Move, 100, 200));

        assert!(matches!(key_event, InputEvent::Key(_)));
        assert!(matches!(mouse_event, InputEvent::Mouse(_)));
    }

    /// Test DeviceType enum
    #[test]
    fn test_device_type_enum() {
        let keyboard = DeviceType::Keyboard;
        let mouse = DeviceType::Mouse;

        assert!(matches!(keyboard, DeviceType::Keyboard));
        assert!(matches!(mouse, DeviceType::Mouse));
    }

    /// Test ModifierState default values
    #[test]
    fn test_modifier_state_default() {
        let state = ModifierState::default();

        assert!(!state.shift);
        assert!(!state.ctrl);
        assert!(!state.alt);
        assert!(!state.meta);
    }

    /// Test ModifierState from virtual key - Shift
    #[test]
    fn test_modifier_state_from_vk_shift() {
        // VK_SHIFT = 0x10
        let (state, pressed) = ModifierState::from_virtual_key(0x10, true).unwrap();
        assert!(state.shift);
        assert!(!state.ctrl);
        assert!(!state.alt);
        assert!(!state.meta);
        assert!(pressed);

        // VK_LSHIFT = 0xA0
        let (state, _) = ModifierState::from_virtual_key(0xA0, true).unwrap();
        assert!(state.shift);

        // VK_RSHIFT = 0xA1
        let (state, _) = ModifierState::from_virtual_key(0xA1, true).unwrap();
        assert!(state.shift);

        // Release state
        let (_, pressed) = ModifierState::from_virtual_key(0x10, false).unwrap();
        assert!(!pressed);
    }

    /// Test ModifierState from virtual key - Control
    #[test]
    fn test_modifier_state_from_vk_control() {
        // VK_CONTROL = 0x11
        let (state, pressed) = ModifierState::from_virtual_key(0x11, true).unwrap();
        assert!(!state.shift);
        assert!(state.ctrl);
        assert!(!state.alt);
        assert!(!state.meta);
        assert!(pressed);

        // VK_LCONTROL = 0xA2
        let (state, _) = ModifierState::from_virtual_key(0xA2, true).unwrap();
        assert!(state.ctrl);

        // VK_RCONTROL = 0xA3
        let (state, _) = ModifierState::from_virtual_key(0xA3, true).unwrap();
        assert!(state.ctrl);
    }

    /// Test ModifierState from virtual key - Alt
    #[test]
    fn test_modifier_state_from_vk_alt() {
        // VK_MENU = 0x12
        let (state, pressed) = ModifierState::from_virtual_key(0x12, true).unwrap();
        assert!(!state.shift);
        assert!(!state.ctrl);
        assert!(state.alt);
        assert!(!state.meta);
        assert!(pressed);

        // VK_LMENU = 0xA4
        let (state, _) = ModifierState::from_virtual_key(0xA4, true).unwrap();
        assert!(state.alt);

        // VK_RMENU = 0xA5
        let (state, _) = ModifierState::from_virtual_key(0xA5, true).unwrap();
        assert!(state.alt);
    }

    /// Test ModifierState from virtual key - Meta/Win
    #[test]
    fn test_modifier_state_from_vk_meta() {
        // VK_LWIN = 0x5B
        let (state, pressed) = ModifierState::from_virtual_key(0x5B, true).unwrap();
        assert!(!state.shift);
        assert!(!state.ctrl);
        assert!(!state.alt);
        assert!(state.meta);
        assert!(pressed);

        // VK_RWIN = 0x5C
        let (state, _) = ModifierState::from_virtual_key(0x5C, true).unwrap();
        assert!(state.meta);
    }

    /// Test ModifierState from virtual key - non-modifier
    #[test]
    fn test_modifier_state_from_vk_non_modifier() {
        // 'A' key = 0x41
        let result = ModifierState::from_virtual_key(0x41, true);
        assert!(result.is_none());

        // '1' key = 0x31
        let result = ModifierState::from_virtual_key(0x31, true);
        assert!(result.is_none());
    }

    /// Test ModifierState merge
    #[test]
    fn test_modifier_state_merge_multiple() {
        let mut state1 = ModifierState::new();
        state1.ctrl = true;

        let mut state2 = ModifierState::new();
        state2.shift = true;

        let mut state3 = ModifierState::new();
        state3.alt = true;

        state1.merge(&state2);
        state1.merge(&state3);

        assert!(state1.ctrl);
        assert!(state1.shift);
        assert!(state1.alt);
        assert!(!state1.meta);
    }

    /// Test complex event sequence
    #[test]
    fn test_event_sequence() {
        let events = vec![
            InputEvent::Key(
                KeyEvent::new(0x1D, 0x11, KeyState::Pressed).with_modifiers(
                    ModifierState {
                        ctrl: true,
                        ..Default::default()
                    },
                ),
            ), // Ctrl
            InputEvent::Key(
                KeyEvent::new(0x1E, 0x41, KeyState::Pressed).with_modifiers(
                    ModifierState {
                        ctrl: true,
                        ..Default::default()
                    },
                ),
            ), // 'A'
            InputEvent::Key(
                KeyEvent::new(0x1E, 0x41, KeyState::Released).with_modifiers(
                    ModifierState {
                        ctrl: true,
                        ..Default::default()
                    },
                ),
            ), // 'A' release
            InputEvent::Key(KeyEvent::new(0x1D, 0x11, KeyState::Released)), // Ctrl release
        ];

        assert_eq!(events.len(), 4);
    }

    /// Test timestamp function
    #[test]
    fn test_timestamp() {
        let ts1 = now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = now();

        assert!(ts2 >= ts1);
        assert!(ts2 - ts1 >= 10);
    }

    /// Test mouse button down event
    #[test]
    fn test_mouse_button_down_event() {
        let event =
            MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Left), 100, 100);

        assert!(event.is_button_down(MouseButton::Left));
        assert!(!event.is_button_down(MouseButton::Right));
    }

    /// Test mouse button up event
    #[test]
    fn test_mouse_button_up_event() {
        let event =
            MouseEvent::new(MouseEventType::ButtonUp(MouseButton::Right), 100, 100);

        assert!(event.is_button_up(MouseButton::Right));
        assert!(!event.is_button_up(MouseButton::Left));
    }

    /// Test mouse wheel event
    #[test]
    fn test_mouse_wheel_event() {
        let event_up = MouseEvent::new(MouseEventType::Wheel(120), 100, 100);
        let event_down = MouseEvent::new(MouseEventType::Wheel(-120), 100, 100);

        assert!(matches!(event_up.event_type, MouseEventType::Wheel(120)));
        assert!(matches!(event_down.event_type, MouseEventType::Wheel(-120)));
    }

    /// Test mouse horizontal wheel event
    #[test]
    fn test_mouse_hwheel_event() {
        let event_right = MouseEvent::new(MouseEventType::HWheel(120), 100, 100);
        let event_left = MouseEvent::new(MouseEventType::HWheel(-120), 100, 100);

        assert!(matches!(
            event_right.event_type,
            MouseEventType::HWheel(120)
        ));
        assert!(matches!(
            event_left.event_type,
            MouseEventType::HWheel(-120)
        ));
    }

    /// Test KeyEvent is modifier key
    #[test]
    fn test_key_event_is_modifier() {
        let shift = KeyEvent::new(0x2A, 0x10, KeyState::Pressed);
        let ctrl = KeyEvent::new(0x1D, 0x11, KeyState::Pressed);
        let alt = KeyEvent::new(0x38, 0x12, KeyState::Pressed);
        let win = KeyEvent::new(0x5B, 0x5B, KeyState::Pressed);
        let a_key = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);

        assert!(shift.is_modifier());
        assert!(ctrl.is_modifier());
        assert!(alt.is_modifier());
        assert!(win.is_modifier());
        assert!(!a_key.is_modifier());
    }

    /// Test KeyEvent modifier identifier
    #[test]
    fn test_key_event_modifier_identifier() {
        let shift = KeyEvent::new(0x2A, 0x10, KeyState::Pressed);
        let ctrl = KeyEvent::new(0x1D, 0x11, KeyState::Pressed);
        let alt = KeyEvent::new(0x38, 0x12, KeyState::Pressed);
        let win = KeyEvent::new(0x5B, 0x5B, KeyState::Pressed);
        let a_key = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);

        assert_eq!(shift.modifier_identifier(), Some("Shift"));
        assert_eq!(ctrl.modifier_identifier(), Some("Control"));
        assert_eq!(alt.modifier_identifier(), Some("Alt"));
        assert_eq!(win.modifier_identifier(), Some("Meta"));
        assert_eq!(a_key.modifier_identifier(), None);
    }

    /// Test KeyEvent with_modifiers
    #[test]
    fn test_key_event_with_modifiers() {
        let mut modifiers = ModifierState::new();
        modifiers.ctrl = true;
        modifiers.shift = true;

        let event =
            KeyEvent::new(0x1E, 0x41, KeyState::Pressed).with_modifiers(modifiers);

        assert!(event.modifiers.ctrl);
        assert!(event.modifiers.shift);
        assert!(!event.modifiers.alt);
    }

    /// Test KeyEvent injected
    #[test]
    fn test_key_event_injected() {
        let event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed).injected();

        assert!(event.is_injected);
    }

    /// Test MouseEvent injected
    #[test]
    fn test_mouse_event_injected() {
        let event = MouseEvent::new(MouseEventType::Move, 100, 100).injected();

        assert!(event.is_injected);
    }

    /// Test InputEvent timestamp
    #[test]
    fn test_input_event_timestamp() {
        let key_event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        let mouse_event =
            InputEvent::Mouse(MouseEvent::new(MouseEventType::Move, 100, 100));

        assert!(key_event.timestamp() > 0);
        assert!(mouse_event.timestamp() > 0);
    }

    /// Test InputEvent is_injected
    #[test]
    fn test_input_event_is_injected() {
        let key_event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        let injected_key =
            InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed).injected());

        assert!(!key_event.is_injected());
        assert!(injected_key.is_injected());
    }
}
