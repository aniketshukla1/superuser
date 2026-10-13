use super::*;

/// Perform one action. `shot` provides the coordinate mapping for clicks/moves.
pub fn execute(action: &Action, shot: &Shot) -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default()).map_err(|e| format!("input init failed: {e}"))?;

    let move_to = |enigo: &mut Enigo, action: &Action| -> Result<(), String> {
        if let (Some(x), Some(y)) = (action.x, action.y) {
            let (sx, sy) = shot.to_screen(x, y);
            enigo.move_mouse(sx, sy, Coordinate::Abs).map_err(|e| format!("move failed: {e}"))?;
        }
        Ok(())
    };

    match action.kind.as_str() {
        "mouse_move" => move_to(&mut enigo, action)?,
        "left_click" => {
            move_to(&mut enigo, action)?;
            enigo.button(Button::Left, Direction::Click).map_err(|e| format!("click failed: {e}"))?;
        }
        "right_click" => {
            move_to(&mut enigo, action)?;
            enigo.button(Button::Right, Direction::Click).map_err(|e| format!("click failed: {e}"))?;
        }
        "middle_click" => {
            move_to(&mut enigo, action)?;
            enigo.button(Button::Middle, Direction::Click).map_err(|e| format!("click failed: {e}"))?;
        }
        "double_click" => {
            move_to(&mut enigo, action)?;
            enigo.button(Button::Left, Direction::Click).map_err(|e| e.to_string())?;
            enigo.button(Button::Left, Direction::Click).map_err(|e| e.to_string())?;
        }
        "triple_click" => {
            move_to(&mut enigo, action)?;
            for _ in 0..3 {
                enigo.button(Button::Left, Direction::Click).map_err(|e| e.to_string())?;
            }
        }
        "left_mouse_down" => {
            move_to(&mut enigo, action)?;
            enigo.button(Button::Left, Direction::Press).map_err(|e| e.to_string())?;
        }
        "left_mouse_up" => {
            move_to(&mut enigo, action)?;
            enigo.button(Button::Left, Direction::Release).map_err(|e| e.to_string())?;
        }
        "left_click_drag" => {
            // press at current pos already set by a prior move; drag to x,y
            enigo.button(Button::Left, Direction::Press).map_err(|e| e.to_string())?;
            move_to(&mut enigo, action)?;
            enigo.button(Button::Left, Direction::Release).map_err(|e| e.to_string())?;
        }
        "type" => {
            let text = action.text.as_deref().unwrap_or("");
            enigo.text(text).map_err(|e| format!("type failed: {e}"))?;
        }
        "key" => {
            let combo = action.text.as_deref().unwrap_or("");
            press_combo(&mut enigo, combo)?;
        }
        "scroll" => {
            let amount = action.scroll_amount.unwrap_or(3);
            let (axis, mag) = match action.scroll_direction.as_deref() {
                Some("up") => (Axis::Vertical, -amount),
                Some("down") => (Axis::Vertical, amount),
                Some("left") => (Axis::Horizontal, -amount),
                Some("right") => (Axis::Horizontal, amount),
                _ => (Axis::Vertical, amount),
            };
            move_to(&mut enigo, action)?;
            enigo.scroll(mag, axis).map_err(|e| format!("scroll failed: {e}"))?;
        }
        "wait" | "screenshot" | "cursor_position" => { /* handled by the loop */ }
        other => return Err(format!("unsupported action: {other}")),
    }
    Ok(())
}

/// Parse a key combo like "cmd+a" / "Return" / "ctrl+c" and press it.
pub(super) fn press_combo(enigo: &mut Enigo, combo: &str) -> Result<(), String> {
    let parts: Vec<&str> = combo.split('+').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Ok(());
    }
    let mods: Vec<Key> = parts[..parts.len() - 1].iter().filter_map(|p| modifier(p)).collect();
    let main = key_from(parts[parts.len() - 1]);

    for m in &mods {
        enigo.key(*m, Direction::Press).map_err(|e| e.to_string())?;
    }
    if let Some(k) = main {
        enigo.key(k, Direction::Click).map_err(|e| e.to_string())?;
    }
    for m in mods.iter().rev() {
        enigo.key(*m, Direction::Release).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(super) fn modifier(name: &str) -> Option<Key> {
    match name.to_lowercase().as_str() {
        "cmd" | "command" | "meta" | "super" | "win" => Some(Key::Meta),
        "ctrl" | "control" => Some(Key::Control),
        "alt" | "option" => Some(Key::Alt),
        "shift" => Some(Key::Shift),
        _ => None,
    }
}

pub(super) fn key_from(name: &str) -> Option<Key> {
    match name.to_lowercase().as_str() {
        "return" | "enter" => Some(Key::Return),
        "tab" => Some(Key::Tab),
        "space" => Some(Key::Space),
        "delete" | "backspace" => Some(Key::Backspace),
        "escape" | "esc" => Some(Key::Escape),
        "up" => Some(Key::UpArrow),
        "down" => Some(Key::DownArrow),
        "left" => Some(Key::LeftArrow),
        "right" => Some(Key::RightArrow),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" | "page_up" => Some(Key::PageUp),
        "pagedown" | "page_down" => Some(Key::PageDown),
        s if s.chars().count() == 1 => s.chars().next().map(Key::Unicode),
        _ => None,
    }
}
