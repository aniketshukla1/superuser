use super::*;

/// The assistant a message is addressed to, when it opens with `@id` — ChatGPT-style
/// mentions (owner, 2026-10-13). That assistant answers this one message, in this
/// conversation, as itself; the conversation stays its own assistant's.
///
/// The same rules as `@brain`: the mention has to open the message ("I asked @coder
/// earlier" is a sentence), only a boundary ends the token (`@coders` is not `@coder`),
/// and case doesn't matter. It has to name an assistant that is switched on and isn't
/// this conversation's own — addressing the one you are already talking to is just
/// talking to it. Returns who, and the words after the mention.
pub fn mentioned(reg: &PersonaRegistry, own: &str, text: &str) -> Option<(Coworker, String)> {
    let t = text.trim_start().strip_prefix('@')?;
    let end = t.find(|c: char| c.is_whitespace() || c == ':' || c == ',').unwrap_or(t.len());
    let (handle, rest) = t.split_at(end);
    let m = reg.manifest(&handle.to_ascii_lowercase())?;
    if m.id == own || !reg.is_enabled(&m.id) {
        return None;
    }
    let who = Coworker { id: m.id.clone(), name: m.name.clone(), tagline: m.tagline.clone() };
    Some((who, rest.trim_start_matches([':', ',']).trim().to_string()))
}

/// What the addressed assistant reads: the person's words, and where it is — a
/// conversation with another assistant, whose earlier replies are that one's, not its
/// own. The transcript keeps what the person typed; this rides only the model's copy.
pub fn mention_brief(own: &str, name: &str, asked: &str) -> String {
    let asked = if asked.is_empty() {
        "(Nothing yet: they only called you in. Say in a sentence or two what you can do for them here.)"
    } else {
        asked
    };
    format!(
        "[The person is talking with {own} in this conversation and has addressed this message to you, {name}. \
         Answer it yourself, as {name}, with your own tools. Earlier assistant replies here are {own}'s unless \
         marked with who gave them.]\n\n{asked}"
    )
}
