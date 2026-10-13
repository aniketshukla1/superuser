
/// Which voice a persona speaks in.
///
/// Personas are already first-class here, with their own icons and toolsets. Giving
/// each one a voice makes multi-persona work *audible*: you know whether the coder
/// or the ops assistant answered without looking at the screen. The mapping is
/// deterministic so a persona sounds the same every session — a colleague whose
/// voice changed between conversations would be unsettling.
pub fn voice_for_persona(persona: &str, available: &[String]) -> Option<String> {
    if available.is_empty() {
        return None;
    }
    let key = persona.trim().to_ascii_lowercase();
    let mut hash: u64 = 1469598103934665603; // FNV-1a
    for b in key.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    Some(available[(hash % available.len() as u64) as usize].clone())
}
