use super::super::*;

/// Is the app allowed to control the computer (macOS Accessibility)?
#[tauri::command]
pub fn cu_access() -> bool {
    crate::core::computer::accessibility_trusted()
}

/// Open the system prompt to grant Accessibility, then report the result.
#[tauri::command]
pub fn cu_request_access() -> bool {
    crate::core::computer::prompt_accessibility()
}

/// Ask the running agent loop to stop at the next safe point.
#[tauri::command]
pub fn cu_stop(state: State<AppState>) {
    state.cu_cancel.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Build a human-readable label for an action, for the live log.
pub(in crate::commands) fn cu_describe(action: &str, input: &serde_json::Value) -> String {
    let coord = input["coordinate"].as_array().map(|a| {
        format!("({}, {})", a.first().and_then(|v| v.as_i64()).unwrap_or(0), a.get(1).and_then(|v| v.as_i64()).unwrap_or(0))
    });
    match action {
        "screenshot" => "Looking at the screen".into(),
        "left_click" | "right_click" | "middle_click" | "double_click" | "triple_click" | "mouse_move" => {
            format!("{}{}", action.replace('_', " "), coord.map(|c| format!(" at {c}")).unwrap_or_default())
        }
        "type" => format!("Typing \"{}\"", input["text"].as_str().unwrap_or("")),
        "key" => format!("Pressing {}", input["text"].as_str().unwrap_or("")),
        "scroll" => format!("Scrolling {}", input["scroll_direction"].as_str().unwrap_or("")),
        "wait" => "Waiting".into(),
        other => other.replace('_', " "),
    }
}

/// Translate a Claude `computer` tool input into our action struct.
pub(in crate::commands) fn cu_to_action(action: &str, input: &serde_json::Value) -> crate::core::computer::Action {
    let coord = input["coordinate"].as_array();
    let (x, y) = match coord {
        Some(a) => (a.first().and_then(|v| v.as_f64()), a.get(1).and_then(|v| v.as_f64())),
        None => (None, None),
    };
    crate::core::computer::Action {
        kind: action.to_string(),
        x,
        y,
        text: input["text"].as_str().map(|s| s.to_string()),
        scroll_direction: input["scroll_direction"].as_str().map(|s| s.to_string()),
        scroll_amount: input["scroll_amount"].as_i64().map(|n| n as i32),
    }
}

/// Pull the first `{...}` JSON object out of a model reply (handles ```json fences).
pub(in crate::commands) fn cu_extract_json(s: &str) -> Option<serde_json::Value> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end < start {
        return None;
    }
    serde_json::from_str(s.get(start..=end)?).ok()
}

/// Build our action from a local model's flat JSON ({"action","x","y","text",...}).
pub(in crate::commands) fn cu_local_action(v: &serde_json::Value) -> crate::core::computer::Action {
    crate::core::computer::Action {
        kind: v["action"].as_str().unwrap_or("screenshot").to_string(),
        x: v["x"].as_f64(),
        y: v["y"].as_f64(),
        text: v["text"].as_str().map(|s| s.to_string()),
        scroll_direction: v["scroll_direction"].as_str().map(|s| s.to_string()),
        scroll_amount: v["scroll_amount"].as_i64().map(|n| n as i32),
    }
}

pub(in crate::commands) fn cu_local_describe(v: &serde_json::Value) -> String {
    let action = v["action"].as_str().unwrap_or("");
    match action {
        "type" => format!("Typing \"{}\"", v["text"].as_str().unwrap_or("")),
        "key" => format!("Pressing {}", v["text"].as_str().unwrap_or("")),
        "scroll" => format!("Scrolling {}", v["scroll_direction"].as_str().unwrap_or("")),
        "done" => "Finishing up".into(),
        _ => format!(
            "{} at ({}, {})",
            action.replace('_', " "),
            v["x"].as_i64().unwrap_or(0),
            v["y"].as_i64().unwrap_or(0)
        ),
    }
}

/// Drive the desktop until the task is done, the user stops it, or the step
/// budget is hit. Routes by model: a Claude model uses Anthropic's computer-use
/// tool; any other (Ollama vision) model uses a local JSON-action loop. Streams
/// progress via the `computer-use` event and stops the moment `cu_stop` is called.
#[tauri::command]
pub async fn computer_use_run(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task: String,
    model: Option<String>,
) -> Result<String, String> {
    cu_run(app, &state, task, model).await
}

/// The computer-use loop — callable from the command or an assistant's
/// `computer_use` tool (both hand it `&AppState`).
/// An action as recorded — kept exact, because rounding is the wrong tool here.
///
/// The first version snapped coordinates to a 25-pixel grid, and its own test caught
/// the flaw: 298 lands in the 275 bucket while 300 and 302 land in 300, so three clicks
/// two pixels apart read as two different actions whenever they straddle a grid line.
/// Nearness is compared in [`cu_stalled`] with a tolerance instead.
pub(in crate::commands) fn cu_signature(desc: &str) -> String {
    desc.trim().to_string()
}

/// An action's words and its numbers, separately: "left_click at (412, 300)" is
/// ("left_click at (, )", [412, 300]).
pub(in crate::commands) fn cu_parts(s: &str) -> (String, Vec<i64>) {
    let mut skeleton = String::new();
    let mut nums = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() || (c == '-' && cur.is_empty()) {
            cur.push(c);
        } else {
            if let Ok(n) = cur.parse::<i64>() {
                nums.push(n);
            } else {
                skeleton.push_str(&cur);
            }
            cur.clear();
            skeleton.push(c);
        }
    }
    if let Ok(n) = cur.parse::<i64>() {
        nums.push(n);
    } else {
        skeleton.push_str(&cur);
    }
    (skeleton, nums)
}

/// How far apart two coordinates may be and still be the same target.
pub(in crate::commands) const CU_NEAR_PX: i64 = 15;

/// The last three actions are the same action at (nearly) the same place. Three, not
/// two: a second click on a button that did not respond is a reasonable retry, and a
/// third is a loop.
pub(in crate::commands) fn cu_stalled(recent: &[String]) -> bool {
    let n = recent.len();
    if n < 3 {
        return false;
    }
    let last: Vec<(String, Vec<i64>)> = recent[n - 3..].iter().map(|s| cu_parts(s)).collect();
    let (skel, nums) = &last[0];
    last.iter().all(|(s, v)| {
        s == skel && v.len() == nums.len() && v.iter().zip(nums).all(|(a, b)| (a - b).abs() <= CU_NEAR_PX)
    })
}

/// What to tell the calling agent when the loop gave up on itself: honest about the
/// state it left, and naming the move that usually works instead.
pub(in crate::commands) fn cu_stall_report(history: &[String]) -> String {
    let tail: Vec<&String> = history.iter().skip(history.len().saturating_sub(4)).collect();
    format!(
        "Stopped — the same action was repeated three times without the screen changing, which is a \
         grounding loop, not progress. The desktop is as the last attempt left it. Recent actions:\n{}\n\
         A file, a shell command or the app's own tool is usually more reliable than clicking for this.",
        tail.iter().map(|s| format!("- {s}")).collect::<Vec<_>>().join("\n")
    )
}

/// Replace `{{N}}` in every string of a plan step's arguments with step N's output.
pub(in crate::commands) fn plan_substitute(v: &serde_json::Value, results: &[String]) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::String(s) => {
            let mut out = s.clone();
            for (i, r) in results.iter().enumerate() {
                out = out.replace(&format!("{{{{{i}}}}}"), r.trim());
            }
            Value::String(out)
        }
        Value::Array(a) => Value::Array(a.iter().map(|x| plan_substitute(x, results)).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), plan_substitute(x, results))).collect()),
        Value::Null => serde_json::json!({}),
        other => other.clone(),
    }
}
