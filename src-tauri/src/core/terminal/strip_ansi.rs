
/// Enough ANSI/VT handling for captured output. Also used by core::devserver,
/// whose log pane is plain text rather than a terminal emulator: CSI/OSC sequences and lone escapes.
pub fn strip_ansi(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == '\x1b' && i + 1 < b.len() {
            match b[i + 1] {
                '[' => {
                    // CSI: params until a final byte in @..~
                    i += 2;
                    while i < b.len() && !('@'..='~').contains(&b[i]) {
                        i += 1;
                    }
                    i += 1;
                }
                ']' => {
                    // OSC: until BEL or ST (ESC \)
                    i += 2;
                    while i < b.len() && b[i] != '\x07' {
                        if b[i] == '\x1b' && i + 1 < b.len() && b[i + 1] == '\\' {
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                    i += 1;
                }
                _ => i += 2,
            }
        } else {
            if b[i] != '\r' {
                out.push(b[i]);
            }
            i += 1;
        }
    }
    out
}
