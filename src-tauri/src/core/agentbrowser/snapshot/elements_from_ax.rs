use super::*;

/// How many elements one snapshot describes. Bounded for the same reason page text is:
/// a real page can have thousands, and all of them go into a model's context.
pub const ELEMENT_LIMIT: usize = 200;

/// The roles worth handing to something that wants to act.
pub const INTERACTIVE: &[&str] = &[
    "button", "checkbox", "combobox", "link", "listbox", "menuitem", "menuitemcheckbox", "menuitemradio", "option",
    "radio", "searchbox", "slider", "spinbutton", "switch", "tab", "textbox",
];

/// Roles whose unticked state means something, so an absent `checked` is false, not unknown.
pub const CHECKABLE: &[&str] = &["checkbox", "menuitemcheckbox", "menuitemradio", "radio", "switch"];

/// Roles a value can be typed into, whose DOM `type` is worth reading (`password`).
pub const TYPABLE: &[&str] = &["textbox", "searchbox", "combobox", "spinbutton"];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Element {
    pub r#ref: String,
    pub role: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// The `type` attribute of an input, lower-cased — `password`, `email`. Empty when
    /// there isn't one. Filled in by the caller, which is the one holding a browser.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub input_type: String,
    #[serde(skip)]
    pub backend_node_id: i64,
}

pub(super) fn text_of(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

pub(super) fn cap(s: &str, n: usize) -> String {
    let t = s.trim();
    if t.chars().count() > n {
        t.chars().take(n).collect()
    } else {
        t.to_string()
    }
}

pub(super) fn property<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    node["properties"].as_array()?.iter().find(|p| p["name"] == name).map(|p| &p["value"]["value"])
}

/// The tree Chrome returned, as a flat list in document order.
///
/// Walked depth first from the root through `childIds`, so the order is the order a
/// person reads the page in, not the order Chrome happened to list the nodes. An ignored
/// node is still descended into: a `generic` wrapper is not actionable, and the button
/// inside it is.
pub fn elements_from_ax(nodes: &[Value]) -> (Vec<Element>, bool) {
    use std::collections::HashMap;
    let by_id: HashMap<&str, &Value> = nodes.iter().filter_map(|n| Some((n["nodeId"].as_str()?, n))).collect();
    let root = nodes
        .iter()
        .find(|n| n.get("parentId").is_none_or(|p| p.is_null()))
        .or_else(|| nodes.first());

    let mut out = Vec::new();
    let mut truncated = false;
    let mut stack: Vec<&Value> = root.into_iter().collect();
    let mut seen = std::collections::HashSet::new();
    while let Some(node) = stack.pop() {
        let id = node["nodeId"].as_str().unwrap_or("");
        if !seen.insert(id.to_string()) {
            continue; // a malformed tree must not loop
        }
        if !node["ignored"].as_bool().unwrap_or(false) {
            if let Some(el) = element(node) {
                if out.len() >= ELEMENT_LIMIT {
                    truncated = true;
                } else {
                    out.push(el);
                }
            }
        }
        if let Some(children) = node["childIds"].as_array() {
            // Reversed onto the stack so the first child is visited first.
            for c in children.iter().rev() {
                if let Some(child) = c.as_str().and_then(|c| by_id.get(c)) {
                    stack.push(child);
                }
            }
        }
    }
    for (i, el) in out.iter_mut().enumerate() {
        el.r#ref = format!("e{}", i + 1);
    }
    (out, truncated)
}

pub(super) fn element(node: &Value) -> Option<Element> {
    let role = node["role"]["value"].as_str()?.to_string();
    if !INTERACTIVE.contains(&role.as_str()) {
        return None;
    }
    // No DOM node, nothing to click — noise in a list whose whole purpose is acting.
    let backend_node_id = node["backendDOMNodeId"].as_i64()?;
    let name = cap(node["name"]["value"].as_str().unwrap_or(""), 200);
    let value = text_of(&node["value"]["value"]).map(|v| cap(&v, 200)).filter(|v| !v.is_empty());
    let disabled = property(node, "disabled").and_then(|v| v.as_bool()).unwrap_or(false);
    // `mixed` is the box above a partly ticked list. It is not ticked, and an assistant told it was
    // left the rows underneath unselected and reported the job done — so "neither" is said
    // as false, which is also the answer that leads to the right click.
    let checked = match property(node, "checked").and_then(text_of).as_deref() {
        Some("true") => Some(true),
        Some(_) => Some(false),
        None if CHECKABLE.contains(&role.as_str()) => Some(false),
        None => None,
    };
    Some(Element { r#ref: String::new(), role, name, value, disabled, checked, input_type: String::new(), backend_node_id })
}

/// One snapshot, as the model reads it: a line per element. Lines rather than JSON —
/// fewer tokens, and small models copy `e7` out of a line more reliably than out of a
/// nested object.
pub fn render(snapshot_id: u64, title: &str, url: &str, elements: &[Element], truncated: bool) -> String {
    let mut out = format!("{title} — {url}\nsnapshot_id: {snapshot_id}\n");
    if elements.is_empty() {
        out.push_str(
            "\nNothing on this page can be clicked or typed into. If it should have a form, the page may still be \
             loading: read it, or take another snapshot in a moment.",
        );
        return out;
    }
    out.push('\n');
    for el in elements {
        let mut line = format!("{} {}", el.r#ref, el.role);
        if !el.name.is_empty() {
            line.push_str(&format!(" \"{}\"", el.name.replace('"', "'")));
        }
        if el.input_type == "password" {
            line.push_str(" [password — use browser_request_secret]");
        }
        if let Some(v) = &el.value {
            // A password field's value is never read back into the model's context.
            if el.input_type != "password" {
                line.push_str(&format!(" = \"{}\"", v.replace('"', "'")));
            }
        }
        match el.checked {
            Some(true) => line.push_str(" [checked]"),
            Some(false) => line.push_str(" [unchecked]"),
            None => {}
        }
        if el.disabled {
            line.push_str(" [disabled]");
        }
        out.push_str(&line);
        out.push('\n');
    }
    if truncated {
        out.push_str(&format!(
            "\n(The first {ELEMENT_LIMIT} are listed. Scroll or open a more specific page to reach the rest.)"
        ));
    }
    out
}
