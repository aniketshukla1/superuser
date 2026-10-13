
/// Distinctive names in a text, lower-cased: capitalised words that do not open a
/// sentence or a line, and `#channel`-style handles.
///
/// Sentence position is the whole trick. "Escalations for anything the Atlas team
/// owns…" capitalises *Escalations* because it starts the sentence and *Atlas*
/// because it is a name, and only the second is worth following. Function words,
/// weekdays and months are excluded because they are capitalised mid-sentence and
/// link everything to everything.
pub fn entities(text: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "and", "but", "for", "with", "from", "this", "that", "these", "those", "they", "their",
        "our", "your", "you", "his", "her", "its", "not", "any", "all", "anyone", "nothing", "none",
        "monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday",
        "january", "february", "march", "april", "may", "june", "july", "august", "september",
        "october", "november", "december",
    ];
    let mut out = Vec::new();
    for line in text.lines() {
        let mut opens = true;
        for raw in line.split_whitespace() {
            let word = raw.trim_matches(|c: char| !(c.is_alphanumeric() || c == '#' || c == '-' || c == '_'));
            let closes = raw.ends_with('.') || raw.ends_with('!') || raw.ends_with('?') || raw.ends_with(':');
            if word.is_empty() {
                continue;
            }
            let low = word.to_lowercase();
            if word.starts_with('#') && word.len() > 2 {
                out.push(low);
            } else if !opens
                && word.chars().next().is_some_and(|c| c.is_uppercase())
                && word.len() >= 3
                && !STOP.contains(&low.as_str())
            {
                out.push(low);
            }
            opens = closes;
        }
    }
    out.sort();
    out.dedup();
    out
}
