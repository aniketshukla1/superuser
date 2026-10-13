//! Key names to the events Chrome needs to act on them.
//!
//! `Input.dispatchKeyEvent` is not satisfied by a key name alone, and there are two
//! ways it goes wrong (after Chrome DevTools' `InputModel`):
//!
//! * an editing key without `windowsVirtualKeyCode` reaches the page's listener and does
//!   nothing — Backspace deletes nothing, Enter submits nothing;
//! * Enter sent as `rawKeyDown` with no text is heard and not acted on. It needs
//!   `keyDown` carrying `"\r"`, which is what a keyboard's Enter carries — measured there
//!   as the sign-in at the end of almost every takeover not going through.

/// One key press, ready for `Input.dispatchKeyEvent`.
#[derive(Debug, Clone, PartialEq)]
pub struct Press {
    pub key: String,
    pub code: String,
    pub vk: i64,
    /// The character this key inserts, if any. Its presence is what makes `keyDown` rather
    /// than `rawKeyDown`.
    pub text: Option<String>,
    /// Alt 1, Control 2, Meta 4, Shift 8 — CDP's bits.
    pub modifiers: i64,
}

fn named(key: &str) -> Option<(&'static str, &'static str, i64)> {
    Some(match key.to_ascii_lowercase().as_str() {
        "enter" | "return" => ("Enter", "Enter", 13),
        "numpadenter" => ("Enter", "NumpadEnter", 13),
        "tab" => ("Tab", "Tab", 9),
        "escape" | "esc" => ("Escape", "Escape", 27),
        "backspace" => ("Backspace", "Backspace", 8),
        "delete" | "del" => ("Delete", "Delete", 46),
        "space" | " " => (" ", "Space", 32),
        "arrowup" | "up" => ("ArrowUp", "ArrowUp", 38),
        "arrowdown" | "down" => ("ArrowDown", "ArrowDown", 40),
        "arrowleft" | "left" => ("ArrowLeft", "ArrowLeft", 37),
        "arrowright" | "right" => ("ArrowRight", "ArrowRight", 39),
        "home" => ("Home", "Home", 36),
        "end" => ("End", "End", 35),
        "pageup" => ("PageUp", "PageUp", 33),
        "pagedown" => ("PageDown", "PageDown", 34),
        _ => return None,
    })
}

/// Read `Enter`, `a`, `Control+A`, `Shift+Tab`. `None` for a name that is not a key.
pub fn press(spec: &str) -> Option<Press> {
    let spec = spec.trim();
    if spec.is_empty() {
        return None;
    }
    // `+` on its own is a key, and so is the last part of `Control++`.
    let (mods, key) = match spec.rsplit_once('+') {
        Some((m, k)) if !k.is_empty() && !m.is_empty() => (m, k),
        Some((m, "")) if m.ends_with('+') || m.is_empty() => (m.trim_end_matches('+'), "+"),
        _ => ("", spec),
    };
    let mut modifiers = 0;
    for m in mods.split('+').filter(|m| !m.is_empty()) {
        modifiers |= match m.to_ascii_lowercase().as_str() {
            "alt" | "option" => 1,
            "control" | "ctrl" => 2,
            "meta" | "cmd" | "command" => 4,
            "shift" => 8,
            _ => return None,
        };
    }
    if let Some((key, code, vk)) = named(key) {
        let text = match (key, modifiers) {
            ("Enter", 0) | ("Enter", 8) => Some("\r".to_string()),
            (" ", 0) | (" ", 8) => Some(" ".to_string()),
            _ => None,
        };
        return Some(Press { key: key.into(), code: code.into(), vk, text, modifiers });
    }
    let mut chars = key.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None; // not a key name we know, and not one character
    }
    let upper = c.to_ascii_uppercase();
    let (code, vk) = if c.is_ascii_alphabetic() {
        (format!("Key{upper}"), upper as i64)
    } else if c.is_ascii_digit() {
        (format!("Digit{c}"), c as i64)
    } else {
        let vk = match c {
            ';' => 186,
            '=' => 187,
            ',' => 188,
            '-' => 189,
            '.' => 190,
            '/' => 191,
            '`' => 192,
            '[' => 219,
            '\\' => 220,
            ']' => 221,
            '\'' => 222,
            _ => 0,
        };
        (String::new(), vk)
    };
    // A shortcut (Control+A) inserts nothing; a plain character does.
    let text = (modifiers & (1 | 2 | 4) == 0).then(|| c.to_string());
    Some(Press { key: c.to_string(), code, vk, text, modifiers })
}

/// Whether a key press presses a thing — the gateway's `activate` intent. Enter and Space
/// activate a focused control; everything else types: a form has three doors.
pub fn activates(spec: &str) -> bool {
    press(spec).is_some_and(|p| p.key == "Enter" || p.key == " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_carries_a_carriage_return_so_chrome_acts_on_it() {
        let p = press("Enter").unwrap();
        assert_eq!((p.key.as_str(), p.vk, p.text.as_deref()), ("Enter", 13, Some("\r")));
        assert_eq!(press("return").unwrap().code, "Enter");
    }

    #[test]
    fn editing_keys_have_codes_and_no_text() {
        let p = press("Backspace").unwrap();
        assert_eq!((p.vk, p.text), (8, None));
        assert_eq!(press("Tab").unwrap().vk, 9);
    }

    #[test]
    fn characters_and_shortcuts() {
        let a = press("a").unwrap();
        assert_eq!((a.code.as_str(), a.vk, a.text.as_deref()), ("KeyA", 65, Some("a")));
        let select_all = press("Control+A").unwrap();
        assert_eq!((select_all.modifiers, select_all.text), (2, None), "a shortcut inserts nothing");
        assert_eq!(press("Shift+Tab").unwrap().modifiers, 8);
        assert_eq!(press("+").unwrap().key, "+");
        assert_eq!(press("Control++").unwrap().key, "+");
        assert!(press("Hyper+A").is_none());
        assert!(press("NotAKey").is_none());
        assert!(press("").is_none());
    }

    #[test]
    fn only_enter_and_space_activate() {
        assert!(activates("Enter"));
        assert!(activates("space"));
        assert!(activates("NumpadEnter"));
        assert!(!activates("Tab"));
        assert!(!activates("a"));
    }
}
