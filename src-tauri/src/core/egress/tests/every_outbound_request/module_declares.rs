use super::*;

/// Whether a module this file belongs to says `egress::allow`. A module split into a
/// folder declares itself once, in `proxy.rs` say, while `proxy/tests.rs` opens the
/// socket: the declaration covers the module, not just the file it is written in.
pub(super) fn module_declares(file: &Path) -> bool {
    fn read_all(dir: &Path, out: &mut String) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                read_all(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push_str(&std::fs::read_to_string(&p).unwrap_or_default());
            }
        }
    }
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    file.ancestors().skip(1).take_while(|a| a.starts_with(&src) && *a != src).any(|dir| {
        let root = dir.with_extension("rs");
        let Ok(mut text) = std::fs::read_to_string(&root) else { return false };
        read_all(dir, &mut text);
        text.contains("egress::allow")
    })
}
