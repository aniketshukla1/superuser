//! todo list (drives the Progress panel)

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    pub status: String, // "pending" | "in_progress" | "done"
}

/// Normalize a model-supplied todo list, coercing the common "completed" alias to
/// "done" and any unknown status to "pending". Accepts array items that are either
/// objects or bare strings.
pub fn normalize_todos(raw: &Value) -> Vec<TodoItem> {
    let arr = raw.as_array().cloned().unwrap_or_default();
    arr.into_iter()
        .map(|entry| {
            if let Some(obj) = entry.as_object() {
                let content = obj.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let mut status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string();
                if status == "completed" {
                    status = "done".into();
                }
                if !matches!(status.as_str(), "pending" | "in_progress" | "done") {
                    status = "pending".into();
                }
                TodoItem { content, status }
            } else {
                TodoItem { content: entry.as_str().unwrap_or("").to_string(), status: "pending".into() }
            }
        })
        .collect()
}
