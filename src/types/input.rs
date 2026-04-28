use super::{now, DeviceType, KeyState, ModifierState, Timestamp, VirtualKey};
use serde::{Deserialize, Serialize};

/// Keyboard event
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    /// Check if this is a modifier key (Shift, Ctrl, Alt, or Meta/Win)
    ///
    /// Modifier keys are typically used in combination with other keys
    /// and are tracked separately in the `modifiers` field.
    pub fn is_modifier(&self) -> bool {
        VirtualKey::new(self.virtual_key).is_modifier()
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MouseEventType {
    /// Mouse move
    ///
    /// The `relative` field indicates whether the movement is relative to the
    /// current cursor position (true) or absolute screen coordinates (false).
    Move { relative: bool },
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

    /// Check if this is a button press (down) event for the specified button
    ///
    /// # Examples
    ///
    /// ```
    /// use wakem::types::{MouseEvent, MouseEventType, MouseButton};
    ///
    /// let event = MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Left), 100, 100);
    /// assert!(event.is_button_down(MouseButton::Left));
    /// assert!(!event.is_button_down(MouseButton::Right));
    /// ```
    pub fn is_button_down(&self, button: MouseButton) -> bool {
        matches!(&self.event_type, MouseEventType::ButtonDown(b) if *b == button)
    }
}

/// Input event (keyboard or mouse)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    /// Get a human-readable event type name for logging and debugging
    ///
    /// Returns "key" for keyboard events and "mouse" for mouse events.
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
        let event = MouseEvent::new(MouseEventType::Move { relative: false }, 100, 200);

        assert_eq!(event.x, 100);
        assert_eq!(event.y, 200);
        assert!(matches!(
            event.event_type,
            MouseEventType::Move { relative: false }
        ));
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

        let mouse_event = InputEvent::Mouse(MouseEvent::new(
            MouseEventType::Move { relative: false },
            100,
            200,
        ));

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

    /// Test complex event sequence
    #[test]
    fn test_event_sequence() {
        let mut ctrl_press = KeyEvent::new(0x1D, 0x11, KeyState::Pressed);
        ctrl_press.modifiers.ctrl = true;
        let mut a_press = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
        a_press.modifiers.ctrl = true;
        let mut a_release = KeyEvent::new(0x1E, 0x41, KeyState::Released);
        a_release.modifiers.ctrl = true;
        let ctrl_release = KeyEvent::new(0x1D, 0x11, KeyState::Released);

        let events = [
            InputEvent::Key(ctrl_press),   // Ctrl
            InputEvent::Key(a_press),      // 'A'
            InputEvent::Key(a_release),    // 'A' release
            InputEvent::Key(ctrl_release), // Ctrl release
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

    /// Test KeyEvent with_modifiers
    #[test]
    fn test_key_event_with_modifiers() {
        let mut event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
        event.modifiers.ctrl = true;
        event.modifiers.shift = true;

        assert!(event.modifiers.ctrl);
        assert!(event.modifiers.shift);
        assert!(!event.modifiers.alt);
    }

    /// Test InputEvent timestamp
    #[test]
    fn test_input_event_timestamp() {
        let key_event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        let mouse_event = InputEvent::Mouse(MouseEvent::new(
            MouseEventType::Move { relative: false },
            100,
            100,
        ));

        assert!(key_event.timestamp() > 0);
        assert!(mouse_event.timestamp() > 0);
    }
}
