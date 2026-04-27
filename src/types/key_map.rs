use keycode::{KeyMap, KeyMappingCode};
use once_cell::sync::Lazy;
use std::collections::HashMap;

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
        KeyMappingCode::KeyA => 0x41,
        KeyMappingCode::KeyB => 0x42,
        KeyMappingCode::KeyC => 0x43,
        KeyMappingCode::KeyD => 0x44,
        KeyMappingCode::KeyE => 0x45,
        KeyMappingCode::KeyF => 0x46,
        KeyMappingCode::KeyG => 0x47,
        KeyMappingCode::KeyH => 0x48,
        KeyMappingCode::KeyI => 0x49,
        KeyMappingCode::KeyJ => 0x4A,
        KeyMappingCode::KeyK => 0x4B,
        KeyMappingCode::KeyL => 0x4C,
        KeyMappingCode::KeyM => 0x4D,
        KeyMappingCode::KeyN => 0x4E,
        KeyMappingCode::KeyO => 0x4F,
        KeyMappingCode::KeyP => 0x50,
        KeyMappingCode::KeyQ => 0x51,
        KeyMappingCode::KeyR => 0x52,
        KeyMappingCode::KeyS => 0x53,
        KeyMappingCode::KeyT => 0x54,
        KeyMappingCode::KeyU => 0x55,
        KeyMappingCode::KeyV => 0x56,
        KeyMappingCode::KeyW => 0x57,
        KeyMappingCode::KeyX => 0x58,
        KeyMappingCode::KeyY => 0x59,
        KeyMappingCode::KeyZ => 0x5A,
        KeyMappingCode::Digit1 => 0x31,
        KeyMappingCode::Digit2 => 0x32,
        KeyMappingCode::Digit3 => 0x33,
        KeyMappingCode::Digit4 => 0x34,
        KeyMappingCode::Digit5 => 0x35,
        KeyMappingCode::Digit6 => 0x36,
        KeyMappingCode::Digit7 => 0x37,
        KeyMappingCode::Digit8 => 0x38,
        KeyMappingCode::Digit9 => 0x39,
        KeyMappingCode::Digit0 => 0x30,
        KeyMappingCode::Enter => 0x0D,
        KeyMappingCode::Escape => 0x1B,
        KeyMappingCode::Backspace => 0x08,
        KeyMappingCode::Tab => 0x09,
        KeyMappingCode::Space => 0x20,
        KeyMappingCode::Minus => 0xBD,
        KeyMappingCode::Equal => 0xBB,
        KeyMappingCode::BracketLeft => 0xDB,
        KeyMappingCode::BracketRight => 0xDD,
        KeyMappingCode::Backslash => 0xDC,
        KeyMappingCode::Semicolon => 0xBA,
        KeyMappingCode::Quote => 0xDE,
        KeyMappingCode::Backquote => 0xC0,
        KeyMappingCode::Comma => 0xBC,
        KeyMappingCode::Period => 0xBE,
        KeyMappingCode::Slash => 0xBF,
        KeyMappingCode::CapsLock => 0x14,
        KeyMappingCode::F1 => 0x70,
        KeyMappingCode::F2 => 0x71,
        KeyMappingCode::F3 => 0x72,
        KeyMappingCode::F4 => 0x73,
        KeyMappingCode::F5 => 0x74,
        KeyMappingCode::F6 => 0x75,
        KeyMappingCode::F7 => 0x76,
        KeyMappingCode::F8 => 0x77,
        KeyMappingCode::F9 => 0x78,
        KeyMappingCode::F10 => 0x79,
        KeyMappingCode::F11 => 0x7A,
        KeyMappingCode::F12 => 0x7B,
        KeyMappingCode::PrintScreen => 0x2C,
        KeyMappingCode::ScrollLock => 0x91,
        KeyMappingCode::Pause => 0x13,
        KeyMappingCode::Insert => 0x2D,
        KeyMappingCode::Home => 0x24,
        KeyMappingCode::PageUp => 0x21,
        KeyMappingCode::Delete => 0x2E,
        KeyMappingCode::End => 0x23,
        KeyMappingCode::PageDown => 0x22,
        KeyMappingCode::ArrowRight => 0x27,
        KeyMappingCode::ArrowLeft => 0x25,
        KeyMappingCode::ArrowDown => 0x28,
        KeyMappingCode::ArrowUp => 0x26,
        KeyMappingCode::NumLock => 0x90,
        KeyMappingCode::NumpadDivide => 0x6F,
        KeyMappingCode::NumpadMultiply => 0x6A,
        KeyMappingCode::NumpadSubtract => 0x6D,
        KeyMappingCode::NumpadAdd => 0x6B,
        KeyMappingCode::NumpadEnter => 0x0D,
        KeyMappingCode::Numpad1 => 0x61,
        KeyMappingCode::Numpad2 => 0x62,
        KeyMappingCode::Numpad3 => 0x63,
        KeyMappingCode::Numpad4 => 0x64,
        KeyMappingCode::Numpad5 => 0x65,
        KeyMappingCode::Numpad6 => 0x66,
        KeyMappingCode::Numpad7 => 0x67,
        KeyMappingCode::Numpad8 => 0x68,
        KeyMappingCode::Numpad9 => 0x69,
        KeyMappingCode::Numpad0 => 0x60,
        KeyMappingCode::NumpadDecimal => 0x6E,
        KeyMappingCode::ContextMenu => 0x5D,
        KeyMappingCode::ShiftLeft => 0xA0,
        KeyMappingCode::ShiftRight => 0xA1,
        KeyMappingCode::ControlLeft => 0xA2,
        KeyMappingCode::ControlRight => 0xA3,
        KeyMappingCode::AltLeft => 0xA4,
        KeyMappingCode::AltRight => 0xA5,
        KeyMappingCode::MetaLeft => 0x5B,
        KeyMappingCode::MetaRight => 0x5C,
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
