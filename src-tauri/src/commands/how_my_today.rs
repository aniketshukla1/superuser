//! "How my Today looks" (Home brief)

use super::*;

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
pub struct TodayItem {
    #[serde(default)] pub time: String,
    #[serde(default)] pub title: String,
    #[serde(default)] pub detail: String,
}

#[derive(serde::Deserialize, Default)]
pub(super) struct BriefOut {
    #[serde(default)] pub(super) meetings: Vec<TodayItem>,
    #[serde(default)] pub(super) travel: Vec<TodayItem>,
    #[serde(default)] pub(super) tasks: Vec<TodayItem>,
    #[serde(default)] pub(super) summary: String,
}

/// What today looks like: meetings, travel and tasks, extracted from the user's own
/// memories by the local model.
#[tauri::command]
pub async fn today_brief(state: State<'_, AppState>, model: Option<String>) -> Result<serde_json::Value, String> {
    use crate::core::model_router::{self, ChatMessage};
    let mut settings = state.settings.lock().unwrap().clone();
    let prefer_cloud = match model.as_deref().filter(|m| !m.is_empty()) {
        Some(m) if m.starts_with("claude") => { settings.cloud_model = m.to_string(); settings.cloud_enabled = true; settings.local_only = false; true }
        Some(m) => { settings.ollama_model = m.to_string(); false }
        None => false,
    };

    let now = chrono::Local::now();
    let date = now.format("%Y-%m-%d").to_string();
    let pretty = now.format("%A %e %B").to_string().replace("  ", " ");

    // Memories that could describe today: meetings, travel, tasks.
    const HINTS: &[&str] = &["meeting", "call", "standup", "sync", "1:1", "interview", "flight", "boarding",
        "booking", "reservation", "hotel", "train", "trip", "task", "todo", "to-do", "due", "deadline",
        "appointment", "invite", "calendar", "itinerary"];
    let all = state.memory.list();
    let mut pool: Vec<String> = Vec::new();
    for m in all.iter() {
        let hay = format!("{} {} {}", m.title, m.summary, m.body).to_lowercase();
        let calendarish = m.category.eq_ignore_ascii_case("calendar");
        if calendarish || HINTS.iter().any(|h| hay.contains(h)) {
            pool.push(format!("--- {} [{}] ({})\n{}", m.title, m.category, m.date, m.body.chars().take(500).collect::<String>()));
        }
        if pool.len() >= 30 { break; }
    }

    let mut out = BriefOut::default();
    if !pool.is_empty() {
        let system = format!(
            "Today is {pretty} ({date}). From the user's own records below, extract what TODAY looks like.\n\
             Return ONLY JSON (no prose, no markdown fences):\n\
             {{\"meetings\":[{{\"time\":\"09:30\",\"title\":\"...\",\"detail\":\"...\"}}],\
             \"travel\":[{{\"time\":\"\",\"title\":\"...\",\"detail\":\"...\"}}],\
             \"tasks\":[{{\"time\":\"\",\"title\":\"...\",\"detail\":\"...\"}}],\
             \"summary\":\"one short sentence\"}}\n\
             Only include things that are actually for today, or imminent (travel in the next few days, tasks due \
             soon). Leave an array empty when there's nothing — never invent items."
        );
        let messages = vec![
            ChatMessage { role: "system".into(), content: system, images: vec![], docs: vec![] },
            ChatMessage { role: "user".into(), content: pool.join("\n\n").chars().take(18_000).collect::<String>(), images: vec![], docs: vec![] },
        ];
        if let Ok((raw, _)) = model_router::complete(&settings, &messages, model_router::Workload::Foreground(prefer_cloud)).await {
            let cleaned = clean_completion(&raw);
            if let (Some(a), Some(b)) = (cleaned.find('{'), cleaned.rfind('}')) {
                if b > a { out = serde_json::from_str(&cleaned[a..=b]).unwrap_or_default(); }
            }
        }
    }

    let empty = out.meetings.is_empty() && out.travel.is_empty() && out.tasks.is_empty();
    Ok(serde_json::json!({
        "date": date, "pretty": pretty,
        "meetings": out.meetings, "travel": out.travel, "tasks": out.tasks,
        "summary": out.summary, "empty": empty,
    }))
}
