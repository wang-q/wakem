use keycode::{KeyMap, KeyMappingCode};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Virtual key code constants (Windows VK_* values)
// These are used for cross-platform consistency
// ============================================================================

pub const VK_SHIFT: u16 = 0x10;
pub const VK_LSHIFT: u16 = 0xA0;
pub const VK_RSHIFT: u16 = 0xA1;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_LCONTROL: u16 = 0xA2;
pub const VK_RCONTROL: u16 = 0xA3;
pub const VK_ALT: u16 = 0x12;
pub const VK_LALT: u16 = 0xA4;
pub const VK_RALT: u16 = 0xA5;
pub const VK_LMETA: u16 = 0x5B;
pub const VK_RMETA: u16 = 0x5C;

/// Virtual key code (Windows VK_* identifier)
///
/// Characteristics:
/// - 0 means invalid/not specified
/// - Non-zero values represent valid virtual key codes
/// - Provides constant definitions for common keys
///
/// # Platform Compatibility
///
/// This module uses Windows virtual key codes as the internal representation
/// for cross-platform consistency. Platform-specific code is responsible for
/// converting native key codes to/from these Windows VK codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VirtualKey(u16);

// Legacy constants for backward compatibility (Windows-specific)
pub const SCAN_CODE_CTRL: u16 = 0x1D;
pub const SCAN_CODE_SHIFT: u16 = 0x2A;
pub const SCAN_CODE_ALT: u16 = 0x38;
pub const SCAN_CODE_META: u16 = 0x5B;

impl VirtualKey {
    pub fn new(key: u16) -> Self {
        VirtualKey(key)
    }

    pub fn value(&self) -> u16 {
        self.0
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    pub fn is_modifier(&self) -> bool {
        matches!(
            self.0,
            VK_SHIFT
                | VK_LSHIFT
                | VK_RSHIFT
                | VK_CONTROL
                | VK_LCONTROL
                | VK_RCONTROL
                | VK_ALT
                | VK_LALT
                | VK_RALT
                | VK_LMETA
                | VK_RMETA
        )
    }

    pub fn modifier_name(&self) -> Option<&'static str> {
        match self.0 {
            VK_SHIFT | VK_LSHIFT | VK_RSHIFT => Some("Shift"),
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL => Some("Control"),
            VK_ALT | VK_LALT | VK_RALT => Some("Alt"),
            VK_LMETA | VK_RMETA => Some("Meta"),
            _ => None,
        }
    }

    // === Common Key Constants ===
    pub const BACKSPACE: Self = VirtualKey(0x08);
    pub const TAB: Self = VirtualKey(0x09);
    pub const ENTER: Self = VirtualKey(0x0D);
    pub const ESCAPE: Self = VirtualKey(0x1B);
    pub const SPACE: Self = VirtualKey(0x20);
    pub const CAPSLOCK: Self = VirtualKey(0x14);

    // === Letter Keys ===
    pub const A: Self = VirtualKey(0x41);
    pub const B: Self = VirtualKey(0x42);
    pub const C: Self = VirtualKey(0x43);
    pub const D: Self = VirtualKey(0x44);
    pub const E: Self = VirtualKey(0x45);
    pub const F: Self = VirtualKey(0x46);
    pub const G: Self = VirtualKey(0x47);
    pub const H: Self = VirtualKey(0x48);
    pub const I: Self = VirtualKey(0x49);
    pub const J: Self = VirtualKey(0x4A);
    pub const K: Self = VirtualKey(0x4B);
    pub const L: Self = VirtualKey(0x4C);
    pub const M: Self = VirtualKey(0x4D);
    pub const N: Self = VirtualKey(0x4E);
    pub const O: Self = VirtualKey(0x4F);
    pub const P: Self = VirtualKey(0x50);
    pub const Q: Self = VirtualKey(0x51);
    pub const R: Self = VirtualKey(0x52);
    pub const S: Self = VirtualKey(0x53);
    pub const T: Self = VirtualKey(0x54);
    pub const U: Self = VirtualKey(0x55);
    pub const V: Self = VirtualKey(0x56);
    pub const W: Self = VirtualKey(0x57);
    pub const X: Self = VirtualKey(0x58);
    pub const Y: Self = VirtualKey(0x59);
    pub const Z: Self = VirtualKey(0x5A);

    // === Number Keys ===
    pub const KEY_0: Self = VirtualKey(0x30);
    pub const KEY_1: Self = VirtualKey(0x31);
    pub const KEY_2: Self = VirtualKey(0x32);
    pub const KEY_3: Self = VirtualKey(0x33);
    pub const KEY_4: Self = VirtualKey(0x34);
    pub const KEY_5: Self = VirtualKey(0x35);
    pub const KEY_6: Self = VirtualKey(0x36);
    pub const KEY_7: Self = VirtualKey(0x37);
    pub const KEY_8: Self = VirtualKey(0x38);
    pub const KEY_9: Self = VirtualKey(0x39);

    // === Function Keys ===
    pub const F1: Self = VirtualKey(0x70);
    pub const F2: Self = VirtualKey(0x71);
    pub const F3: Self = VirtualKey(0x72);
    pub const F4: Self = VirtualKey(0x73);
    pub const F5: Self = VirtualKey(0x74);
    pub const F6: Self = VirtualKey(0x75);
    pub const F7: Self = VirtualKey(0x76);
    pub const F8: Self = VirtualKey(0x77);
    pub const F9: Self = VirtualKey(0x78);
    pub const F10: Self = VirtualKey(0x79);
    pub const F11: Self = VirtualKey(0x7A);
    pub const F12: Self = VirtualKey(0x7B);

    // === Modifier Keys ===
    pub const SHIFT: Self = VirtualKey(VK_SHIFT);
    pub const CONTROL: Self = VirtualKey(VK_CONTROL);
    pub const ALT: Self = VirtualKey(VK_ALT);
    pub const META: Self = VirtualKey(VK_LMETA);

    // === Arrow Keys ===
    pub const LEFT: Self = VirtualKey(0x25);
    pub const UP: Self = VirtualKey(0x26);
    pub const RIGHT: Self = VirtualKey(0x27);
    pub const DOWN: Self = VirtualKey(0x28);

    // === Numpad Keys ===
    pub const NUMPAD0: Self = VirtualKey(0x60);
    pub const NUMPAD1: Self = VirtualKey(0x61);
    pub const NUMPAD2: Self = VirtualKey(0x62);
    pub const NUMPAD3: Self = VirtualKey(0x63);
    pub const NUMPAD4: Self = VirtualKey(0x64);
    pub const NUMPAD5: Self = VirtualKey(0x65);
    pub const NUMPAD6: Self = VirtualKey(0x66);
    pub const NUMPAD7: Self = VirtualKey(0x67);
    pub const NUMPAD8: Self = VirtualKey(0x68);
    pub const NUMPAD9: Self = VirtualKey(0x69);
    pub const NUMPAD_MULTIPLY: Self = VirtualKey(0x6A);
    pub const NUMPAD_ADD: Self = VirtualKey(0x6B);
    pub const NUMPAD_SUBTRACT: Self = VirtualKey(0x6D);
    pub const NUMPAD_DECIMAL: Self = VirtualKey(0x6E);
    pub const NUMPAD_DIVIDE: Self = VirtualKey(0x6F);

    // === Editing Keys ===
    pub const INSERT: Self = VirtualKey(0x2D);
    pub const DELETE: Self = VirtualKey(0x2E);
    pub const HOME: Self = VirtualKey(0x24);
    pub const END: Self = VirtualKey(0x23);
    pub const PAGE_UP: Self = VirtualKey(0x21);
    pub const PAGE_DOWN: Self = VirtualKey(0x22);
    pub const PRINT_SCREEN: Self = VirtualKey(0x2C);
    pub const SCROLL_LOCK: Self = VirtualKey(0x91);
    pub const PAUSE: Self = VirtualKey(0x13);
    pub const NUM_LOCK: Self = VirtualKey(0x90);
    pub const CONTEXT_MENU: Self = VirtualKey(0x5D);

    // === OEM Punctuation Keys ===
    pub const OEM_MINUS: Self = VirtualKey(0xBD);
    pub const OEM_PLUS: Self = VirtualKey(0xBB);
    pub const OEM_1: Self = VirtualKey(0xBA);
    pub const OEM_2: Self = VirtualKey(0xBF);
    pub const OEM_3: Self = VirtualKey(0xC0);
    pub const OEM_4: Self = VirtualKey(0xDB);
    pub const OEM_5: Self = VirtualKey(0xDC);
    pub const OEM_6: Self = VirtualKey(0xDD);
    pub const OEM_7: Self = VirtualKey(0xDE);
    pub const OEM_COMMA: Self = VirtualKey(0xBC);
    pub const OEM_PERIOD: Self = VirtualKey(0xBE);
}

impl fmt::Display for VirtualKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.modifier_name() {
            write!(f, "{}", name)
        } else if self.is_valid() {
            write!(f, "VK_0x{:02X}", self.value())
        } else {
            write!(f, "Invalid")
        }
    }
}

impl From<u16> for VirtualKey {
    fn from(value: u16) -> Self {
        VirtualKey::new(value)
    }
}

impl From<VirtualKey> for u16 {
    fn from(vk: VirtualKey) -> u16 {
        vk.value()
    }
}

// ============================================================================
// Key name parsing (using keycode crate for scan codes + alias resolution)
// ============================================================================

static ALIAS_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    m.insert("caps", "CapsLock");
    m.insert("backtick", "Backquote");
    m.insert("grave", "Backquote");
    m.insert("return", "Enter");
    m.insert("esc", "Escape");
    m.insert("del", "Delete");
    m.insert("forwarddelete", "Delete");
    m.insert("forwarddel", "Delete");
    m.insert("ins", "Insert");
    m.insert("pgup", "PageUp");
    m.insert("pgdn", "PageDown");
    m.insert("pagedown", "PageDown");
    m.insert("home", "Home");
    m.insert("end", "End");
    m.insert("backspace", "Backspace");
    m.insert("space", "Space");
    m.insert("tab", "Tab");
    m.insert("enter", "Enter");
    m.insert("escape", "Escape");
    m.insert("delete", "Delete");
    m.insert("insert", "Insert");
    m.insert("pageup", "PageUp");
    m.insert("capslock", "CapsLock");
    m.insert("printscreen", "PrintScreen");
    m.insert("scrolllock", "ScrollLock");
    m.insert("pause", "Pause");
    m.insert("numlock", "NumLock");
    m.insert("contextmenu", "ContextMenu");

    m.insert("left", "ArrowLeft");
    m.insert("up", "ArrowUp");
    m.insert("right", "ArrowRight");
    m.insert("down", "ArrowDown");

    m.insert("lshift", "ShiftLeft");
    m.insert("leftshift", "ShiftLeft");
    m.insert("rshift", "ShiftRight");
    m.insert("rightshift", "ShiftRight");
    m.insert("lctrl", "ControlLeft");
    m.insert("lcontrol", "ControlLeft");
    m.insert("leftctrl", "ControlLeft");
    m.insert("leftcontrol", "ControlLeft");
    m.insert("rctrl", "ControlRight");
    m.insert("rcontrol", "ControlRight");
    m.insert("rightctrl", "ControlRight");
    m.insert("rightcontrol", "ControlRight");
    m.insert("lalt", "AltLeft");
    m.insert("leftalt", "AltLeft");
    m.insert("ralt", "AltRight");
    m.insert("rightalt", "AltRight");
    m.insert("lwin", "MetaLeft");
    m.insert("lmeta", "MetaLeft");
    m.insert("leftwin", "MetaLeft");
    m.insert("leftmeta", "MetaLeft");
    m.insert("rwin", "MetaRight");
    m.insert("rmeta", "MetaRight");
    m.insert("rightwin", "MetaRight");
    m.insert("rightmeta", "MetaRight");

    m.insert("comma", "Comma");
    m.insert("period", "Period");
    m.insert("semicolon", "Semicolon");
    m.insert("quote", "Quote");
    m.insert("apostrophe", "Quote");
    m.insert("bracketleft", "BracketLeft");
    m.insert("bracketright", "BracketRight");
    m.insert("backslash", "Backslash");
    m.insert("minus", "Minus");
    m.insert("equal", "Equal");
    m.insert("slash", "Slash");

    m.insert("numpad0", "Numpad0");
    m.insert("num0", "Numpad0");
    m.insert("numpad1", "Numpad1");
    m.insert("num1", "Numpad1");
    m.insert("numpad2", "Numpad2");
    m.insert("num2", "Numpad2");
    m.insert("numpad3", "Numpad3");
    m.insert("num3", "Numpad3");
    m.insert("numpad4", "Numpad4");
    m.insert("num4", "Numpad4");
    m.insert("numpad5", "Numpad5");
    m.insert("num5", "Numpad5");
    m.insert("numpad6", "Numpad6");
    m.insert("num6", "Numpad6");
    m.insert("numpad7", "Numpad7");
    m.insert("num7", "Numpad7");
    m.insert("numpad8", "Numpad8");
    m.insert("num8", "Numpad8");
    m.insert("numpad9", "Numpad9");
    m.insert("num9", "Numpad9");
    m.insert("numpaddot", "NumpadDecimal");
    m.insert("numdot", "NumpadDecimal");
    m.insert("numpaddecimal", "NumpadDecimal");
    m.insert("numpadenter", "NumpadEnter");
    m.insert("numenter", "NumpadEnter");
    m.insert("numpadadd", "NumpadAdd");
    m.insert("numplus", "NumpadAdd");
    m.insert("numpadsub", "NumpadSubtract");
    m.insert("numminus", "NumpadSubtract");
    m.insert("numpadmul", "NumpadMultiply");
    m.insert("nummul", "NumpadMultiply");
    m.insert("numpadmultiply", "NumpadMultiply");
    m.insert("numpaddiv", "NumpadDivide");
    m.insert("numslash", "NumpadDivide");
    m.insert("numpaddivide", "NumpadDivide");

    m.insert("f1", "F1");
    m.insert("f2", "F2");
    m.insert("f3", "F3");
    m.insert("f4", "F4");
    m.insert("f5", "F5");
    m.insert("f6", "F6");
    m.insert("f7", "F7");
    m.insert("f8", "F8");
    m.insert("f9", "F9");
    m.insert("f10", "F10");
    m.insert("f11", "F11");
    m.insert("f12", "F12");
    m.insert("f13", "F13");
    m.insert("f14", "F14");
    m.insert("f15", "F15");
    m.insert("f16", "F16");
    m.insert("f17", "F17");
    m.insert("f18", "F18");
    m.insert("f19", "F19");
    m.insert("f20", "F20");
    m.insert("f21", "F21");
    m.insert("f22", "F22");
    m.insert("f23", "F23");
    m.insert("f24", "F24");

    m
});

static CHAR_TO_W3: Lazy<HashMap<char, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for c in 'a'..='z' {
        let name = match c {
            'a' => "KeyA",
            'b' => "KeyB",
            'c' => "KeyC",
            'd' => "KeyD",
            'e' => "KeyE",
            'f' => "KeyF",
            'g' => "KeyG",
            'h' => "KeyH",
            'i' => "KeyI",
            'j' => "KeyJ",
            'k' => "KeyK",
            'l' => "KeyL",
            'm' => "KeyM",
            'n' => "KeyN",
            'o' => "KeyO",
            'p' => "KeyP",
            'q' => "KeyQ",
            'r' => "KeyR",
            's' => "KeyS",
            't' => "KeyT",
            'u' => "KeyU",
            'v' => "KeyV",
            'w' => "KeyW",
            'x' => "KeyX",
            'y' => "KeyY",
            'z' => "KeyZ",
            _ => unreachable!(),
        };
        m.insert(c, name);
    }
    for d in '0'..='9' {
        let name = match d {
            '0' => "Digit0",
            '1' => "Digit1",
            '2' => "Digit2",
            '3' => "Digit3",
            '4' => "Digit4",
            '5' => "Digit5",
            '6' => "Digit6",
            '7' => "Digit7",
            '8' => "Digit8",
            '9' => "Digit9",
            _ => unreachable!(),
        };
        m.insert(d, name);
    }
    m.insert(',', "Comma");
    m.insert('.', "Period");
    m.insert(';', "Semicolon");
    m.insert('\'', "Quote");
    m.insert('[', "BracketLeft");
    m.insert(']', "BracketRight");
    m.insert('\\', "Backslash");
    m.insert('-', "Minus");
    m.insert('=', "Equal");
    m.insert('/', "Slash");
    m.insert('`', "Backquote");
    m
});

fn code_to_vk(code: KeyMappingCode) -> u16 {
    match code {
        KeyMappingCode::KeyA => VirtualKey::A.value(),
        KeyMappingCode::KeyB => VirtualKey::B.value(),
        KeyMappingCode::KeyC => VirtualKey::C.value(),
        KeyMappingCode::KeyD => VirtualKey::D.value(),
        KeyMappingCode::KeyE => VirtualKey::E.value(),
        KeyMappingCode::KeyF => VirtualKey::F.value(),
        KeyMappingCode::KeyG => VirtualKey::G.value(),
        KeyMappingCode::KeyH => VirtualKey::H.value(),
        KeyMappingCode::KeyI => VirtualKey::I.value(),
        KeyMappingCode::KeyJ => VirtualKey::J.value(),
        KeyMappingCode::KeyK => VirtualKey::K.value(),
        KeyMappingCode::KeyL => VirtualKey::L.value(),
        KeyMappingCode::KeyM => VirtualKey::M.value(),
        KeyMappingCode::KeyN => VirtualKey::N.value(),
        KeyMappingCode::KeyO => VirtualKey::O.value(),
        KeyMappingCode::KeyP => VirtualKey::P.value(),
        KeyMappingCode::KeyQ => VirtualKey::Q.value(),
        KeyMappingCode::KeyR => VirtualKey::R.value(),
        KeyMappingCode::KeyS => VirtualKey::S.value(),
        KeyMappingCode::KeyT => VirtualKey::T.value(),
        KeyMappingCode::KeyU => VirtualKey::U.value(),
        KeyMappingCode::KeyV => VirtualKey::V.value(),
        KeyMappingCode::KeyW => VirtualKey::W.value(),
        KeyMappingCode::KeyX => VirtualKey::X.value(),
        KeyMappingCode::KeyY => VirtualKey::Y.value(),
        KeyMappingCode::KeyZ => VirtualKey::Z.value(),
        KeyMappingCode::Digit0 => VirtualKey::KEY_0.value(),
        KeyMappingCode::Digit1 => VirtualKey::KEY_1.value(),
        KeyMappingCode::Digit2 => VirtualKey::KEY_2.value(),
        KeyMappingCode::Digit3 => VirtualKey::KEY_3.value(),
        KeyMappingCode::Digit4 => VirtualKey::KEY_4.value(),
        KeyMappingCode::Digit5 => VirtualKey::KEY_5.value(),
        KeyMappingCode::Digit6 => VirtualKey::KEY_6.value(),
        KeyMappingCode::Digit7 => VirtualKey::KEY_7.value(),
        KeyMappingCode::Digit8 => VirtualKey::KEY_8.value(),
        KeyMappingCode::Digit9 => VirtualKey::KEY_9.value(),
        KeyMappingCode::Enter => VirtualKey::ENTER.value(),
        KeyMappingCode::Escape => VirtualKey::ESCAPE.value(),
        KeyMappingCode::Backspace => VirtualKey::BACKSPACE.value(),
        KeyMappingCode::Tab => VirtualKey::TAB.value(),
        KeyMappingCode::Space => VirtualKey::SPACE.value(),
        KeyMappingCode::CapsLock => VirtualKey::CAPSLOCK.value(),
        KeyMappingCode::F1 => VirtualKey::F1.value(),
        KeyMappingCode::F2 => VirtualKey::F2.value(),
        KeyMappingCode::F3 => VirtualKey::F3.value(),
        KeyMappingCode::F4 => VirtualKey::F4.value(),
        KeyMappingCode::F5 => VirtualKey::F5.value(),
        KeyMappingCode::F6 => VirtualKey::F6.value(),
        KeyMappingCode::F7 => VirtualKey::F7.value(),
        KeyMappingCode::F8 => VirtualKey::F8.value(),
        KeyMappingCode::F9 => VirtualKey::F9.value(),
        KeyMappingCode::F10 => VirtualKey::F10.value(),
        KeyMappingCode::F11 => VirtualKey::F11.value(),
        KeyMappingCode::F12 => VirtualKey::F12.value(),
        KeyMappingCode::PrintScreen => VirtualKey::PRINT_SCREEN.value(),
        KeyMappingCode::ScrollLock => VirtualKey::SCROLL_LOCK.value(),
        KeyMappingCode::Pause => VirtualKey::PAUSE.value(),
        KeyMappingCode::Insert => VirtualKey::INSERT.value(),
        KeyMappingCode::Home => VirtualKey::HOME.value(),
        KeyMappingCode::PageUp => VirtualKey::PAGE_UP.value(),
        KeyMappingCode::Delete => VirtualKey::DELETE.value(),
        KeyMappingCode::End => VirtualKey::END.value(),
        KeyMappingCode::PageDown => VirtualKey::PAGE_DOWN.value(),
        KeyMappingCode::ArrowRight => VirtualKey::RIGHT.value(),
        KeyMappingCode::ArrowLeft => VirtualKey::LEFT.value(),
        KeyMappingCode::ArrowDown => VirtualKey::DOWN.value(),
        KeyMappingCode::ArrowUp => VirtualKey::UP.value(),
        KeyMappingCode::NumLock => VirtualKey::NUM_LOCK.value(),
        KeyMappingCode::ContextMenu => VirtualKey::CONTEXT_MENU.value(),
        KeyMappingCode::Numpad0 => VirtualKey::NUMPAD0.value(),
        KeyMappingCode::Numpad1 => VirtualKey::NUMPAD1.value(),
        KeyMappingCode::Numpad2 => VirtualKey::NUMPAD2.value(),
        KeyMappingCode::Numpad3 => VirtualKey::NUMPAD3.value(),
        KeyMappingCode::Numpad4 => VirtualKey::NUMPAD4.value(),
        KeyMappingCode::Numpad5 => VirtualKey::NUMPAD5.value(),
        KeyMappingCode::Numpad6 => VirtualKey::NUMPAD6.value(),
        KeyMappingCode::Numpad7 => VirtualKey::NUMPAD7.value(),
        KeyMappingCode::Numpad8 => VirtualKey::NUMPAD8.value(),
        KeyMappingCode::Numpad9 => VirtualKey::NUMPAD9.value(),
        KeyMappingCode::NumpadDecimal => VirtualKey::NUMPAD_DECIMAL.value(),
        KeyMappingCode::NumpadMultiply => VirtualKey::NUMPAD_MULTIPLY.value(),
        KeyMappingCode::NumpadSubtract => VirtualKey::NUMPAD_SUBTRACT.value(),
        KeyMappingCode::NumpadAdd => VirtualKey::NUMPAD_ADD.value(),
        KeyMappingCode::NumpadDivide => VirtualKey::NUMPAD_DIVIDE.value(),
        KeyMappingCode::NumpadEnter => VirtualKey::ENTER.value(),
        KeyMappingCode::ShiftLeft => VK_LSHIFT,
        KeyMappingCode::ShiftRight => VK_RSHIFT,
        KeyMappingCode::ControlLeft => VK_LCONTROL,
        KeyMappingCode::ControlRight => VK_RCONTROL,
        KeyMappingCode::AltLeft => VK_LALT,
        KeyMappingCode::AltRight => VK_RALT,
        KeyMappingCode::MetaLeft => VK_LMETA,
        KeyMappingCode::MetaRight => VK_RMETA,
        KeyMappingCode::Minus => VirtualKey::OEM_MINUS.value(),
        KeyMappingCode::Equal => VirtualKey::OEM_PLUS.value(),
        KeyMappingCode::BracketLeft => VirtualKey::OEM_4.value(),
        KeyMappingCode::BracketRight => VirtualKey::OEM_6.value(),
        KeyMappingCode::Backslash => VirtualKey::OEM_5.value(),
        KeyMappingCode::Semicolon => VirtualKey::OEM_1.value(),
        KeyMappingCode::Quote => VirtualKey::OEM_7.value(),
        KeyMappingCode::Backquote => VirtualKey::OEM_3.value(),
        KeyMappingCode::Comma => VirtualKey::OEM_COMMA.value(),
        KeyMappingCode::Period => VirtualKey::OEM_PERIOD.value(),
        KeyMappingCode::Slash => VirtualKey::OEM_2.value(),
        KeyMappingCode::F13 => 0x7C,
        KeyMappingCode::F14 => 0x7D,
        KeyMappingCode::F15 => 0x7E,
        KeyMappingCode::F16 => 0x7F,
        KeyMappingCode::F17 => 0x80,
        KeyMappingCode::F18 => 0x81,
        KeyMappingCode::F19 => 0x82,
        KeyMappingCode::F20 => 0x83,
        KeyMappingCode::F21 => 0x84,
        KeyMappingCode::F22 => 0x85,
        KeyMappingCode::F23 => 0x86,
        KeyMappingCode::F24 => 0x87,
        _ => 0,
    }
}

fn resolve_to_w3_name(input: &str) -> Option<&'static str> {
    if let Some(w3_name) = ALIAS_MAP.get(input) {
        return Some(*w3_name);
    }

    if input.len() == 1 {
        let c = input.chars().next().unwrap();
        if let Some(w3_name) = CHAR_TO_W3.get(&c) {
            return Some(*w3_name);
        }
    }

    if input.parse::<KeyMappingCode>().is_ok() {
        let owned = input.to_owned();
        return Some(Box::leak(owned.into_boxed_str()));
    }

    None
}

fn w3_name_to_key_map(w3_name: &str) -> Option<KeyMap> {
    let code = w3_name.parse::<KeyMappingCode>().ok()?;
    Some(KeyMap::from(code))
}

/// Parse key name to (scan_code, virtual_key)
pub fn parse_key(name: &str) -> anyhow::Result<(u16, u16)> {
    let lower = name.to_lowercase();
    let w3_name = resolve_to_w3_name(&lower)
        .ok_or_else(|| anyhow::anyhow!("Unknown key name: {}", name))?;

    let key_map = w3_name_to_key_map(w3_name)
        .ok_or_else(|| anyhow::anyhow!("No key mapping found for: {}", name))?;

    let scan_code = key_map.win as u16;
    let vk = key_map.code.map(code_to_vk).unwrap_or(0);

    Ok((scan_code, vk))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_key_validity() {
        assert!(!VirtualKey::new(0).is_valid());
        assert!(VirtualKey::new(0x41).is_valid());
    }

    #[test]
    fn test_virtual_key_modifiers() {
        assert!(VirtualKey::SHIFT.is_modifier());
        assert!(VirtualKey::CONTROL.is_modifier());
        assert!(VirtualKey::ALT.is_modifier());
        assert!(!VirtualKey::A.is_modifier());
    }

    #[test]
    fn test_virtual_key_constants() {
        assert_eq!(VirtualKey::A.value(), 0x41);
        assert_eq!(VirtualKey::ENTER.value(), 0x0D);
        assert_eq!(VirtualKey::F1.value(), 0x70);
        assert_eq!(VirtualKey::NUMPAD0.value(), 0x60);
        assert_eq!(VirtualKey::OEM_MINUS.value(), 0xBD);
        assert_eq!(VirtualKey::INSERT.value(), 0x2D);
        assert_eq!(VirtualKey::DELETE.value(), 0x2E);
    }

    #[test]
    fn test_conversion() {
        let vk = VirtualKey::from(0x42u16);
        let value: u16 = vk.into();
        assert_eq!(value, 0x42);
    }

    #[test]
    fn test_parse_key_special() {
        assert_eq!(parse_key("capslock").unwrap(), (0x3A, 0x14));
        assert_eq!(parse_key("caps").unwrap(), (0x3A, 0x14));
        assert_eq!(parse_key("backspace").unwrap(), (0x0E, 0x08));
        assert_eq!(parse_key("enter").unwrap(), (0x1C, 0x0D));
        assert_eq!(parse_key("return").unwrap(), (0x1C, 0x0D));
        assert_eq!(parse_key("escape").unwrap(), (0x01, 0x1B));
        assert_eq!(parse_key("esc").unwrap(), (0x01, 0x1B));
        assert_eq!(parse_key("space").unwrap(), (0x39, 0x20));
        assert_eq!(parse_key("tab").unwrap(), (0x0F, 0x09));
        assert_eq!(parse_key("grave").unwrap(), (0x29, 0xC0));
        assert_eq!(parse_key("backtick").unwrap(), (0x29, 0xC0));
    }

    #[test]
    fn test_parse_key_arrows() {
        assert_eq!(parse_key("left").unwrap(), (0xE04B, 0x25));
        assert_eq!(parse_key("up").unwrap(), (0xE048, 0x26));
        assert_eq!(parse_key("right").unwrap(), (0xE04D, 0x27));
        assert_eq!(parse_key("down").unwrap(), (0xE050, 0x28));
    }

    #[test]
    fn test_parse_key_editing() {
        assert_eq!(parse_key("home").unwrap(), (0xE047, 0x24));
        assert_eq!(parse_key("end").unwrap(), (0xE04F, 0x23));
        assert_eq!(parse_key("pageup").unwrap(), (0xE049, 0x21));
        assert_eq!(parse_key("pagedown").unwrap(), (0xE051, 0x22));
        assert_eq!(parse_key("delete").unwrap(), (0xE053, 0x2E));
        assert_eq!(parse_key("del").unwrap(), (0xE053, 0x2E));
        assert_eq!(parse_key("insert").unwrap(), (0xE052, 0x2D));
        assert_eq!(parse_key("ins").unwrap(), (0xE052, 0x2D));
    }

    #[test]
    fn test_parse_key_modifiers() {
        assert_eq!(parse_key("lshift").unwrap(), (0x2A, 0xA0));
        assert_eq!(parse_key("leftshift").unwrap(), (0x2A, 0xA0));
        assert_eq!(parse_key("rshift").unwrap(), (0x36, 0xA1));
        assert_eq!(parse_key("rightshift").unwrap(), (0x36, 0xA1));
        assert_eq!(parse_key("lctrl").unwrap(), (0x1D, 0xA2));
        assert_eq!(parse_key("leftctrl").unwrap(), (0x1D, 0xA2));
        assert_eq!(parse_key("rctrl").unwrap(), (0xE01D, 0xA3));
        assert_eq!(parse_key("rightctrl").unwrap(), (0xE01D, 0xA3));
        assert_eq!(parse_key("lalt").unwrap(), (0x38, 0xA4));
        assert_eq!(parse_key("leftalt").unwrap(), (0x38, 0xA4));
        assert_eq!(parse_key("ralt").unwrap(), (0xE038, 0xA5));
        assert_eq!(parse_key("rightalt").unwrap(), (0xE038, 0xA5));
        assert_eq!(parse_key("lwin").unwrap(), (0xE05B, 0x5B));
        assert_eq!(parse_key("lmeta").unwrap(), (0xE05B, 0x5B));
        assert_eq!(parse_key("rwin").unwrap(), (0xE05C, 0x5C));
        assert_eq!(parse_key("rmeta").unwrap(), (0xE05C, 0x5C));
    }

    #[test]
    fn test_parse_key_function() {
        assert_eq!(parse_key("f1").unwrap(), (0x3B, 0x70));
        assert_eq!(parse_key("f2").unwrap(), (0x3C, 0x71));
        assert_eq!(parse_key("f12").unwrap(), (0x58, 0x7B));
    }

    #[test]
    fn test_parse_key_letters() {
        assert_eq!(parse_key("a").unwrap(), (0x1E, 0x41));
        assert_eq!(parse_key("z").unwrap(), (0x2C, 0x5A));
    }

    #[test]
    fn test_parse_key_digits() {
        assert_eq!(parse_key("1").unwrap(), (0x02, 0x31));
        assert_eq!(parse_key("0").unwrap(), (0x0B, 0x30));
    }

    #[test]
    fn test_parse_key_punctuation() {
        assert_eq!(parse_key("comma").unwrap(), (0x33, 0xBC));
        assert_eq!(parse_key("period").unwrap(), (0x34, 0xBE));
        assert_eq!(parse_key("semicolon").unwrap(), (0x27, 0xBA));
        assert_eq!(parse_key("minus").unwrap(), (0x0C, 0xBD));
        assert_eq!(parse_key("equal").unwrap(), (0x0D, 0xBB));
    }

    #[test]
    fn test_parse_key_numpad() {
        assert_eq!(parse_key("numpad0").unwrap(), (0x52, 0x60));
        assert_eq!(parse_key("num0").unwrap(), (0x52, 0x60));
        assert_eq!(parse_key("numpad1").unwrap(), (0x4F, 0x61));
        assert_eq!(parse_key("numpadenter").unwrap(), (0xE01C, 0x0D));
        assert_eq!(parse_key("numpadadd").unwrap(), (0x4E, 0x6B));
        assert_eq!(parse_key("numplus").unwrap(), (0x4E, 0x6B));
        assert_eq!(parse_key("numpadsub").unwrap(), (0x4A, 0x6D));
        assert_eq!(parse_key("numpadmul").unwrap(), (0x37, 0x6A));
        assert_eq!(parse_key("numpaddiv").unwrap(), (0xE035, 0x6F));
    }

    #[test]
    fn test_parse_key_unknown() {
        assert!(parse_key("nonexistentkey").is_err());
    }
}
