//! A small view an assistant draws in the conversation.
//!
//! The cards, charts and tables are fixed shapes: four slots, filled by the model, drawn
//! by us. This is the other half — a view the model lays out itself,
//! for the answers whose shape nothing here anticipated.
//!
//! **What makes it safe is what it cannot do.** The HTML is rendered in an iframe with its
//! own origin and no `allow-same-origin`, so it cannot read this app, its storage or its
//! token; a Content-Security-Policy of `default-src 'none'` means it cannot fetch, load an
//! image from a server, or phone anywhere; no `allow-forms`, `allow-popups` or
//! `allow-top-navigation`, so a form in it submits nowhere and a link cannot move the app.
//! It draws, and that is the whole of it.
//!
//! **And it is off until someone turns it on.** Everything above is true, and "a model
//! wrote the page you are looking at" is still a thing to agree to rather than discover.

/// The most HTML one view may carry. A view that needs more than this is a file.
pub const MAX_BYTES: usize = 64 * 1024;

/// What the model is told about the medium. Kept here beside the sandbox it describes, so
/// the promise and the enforcement cannot drift apart.
pub const GUIDANCE: &str = "\n\n## Drawing a view\n\n\
You can draw a small view in the conversation with `ui_component` when the answer has a shape none of \
the fixed cards fit — a comparison someone will scan, a diagram, a layout with its own logic. Prefer \
`card`, `table` and `chart` when one of them fits: they are shorter to write and they already look \
like the rest of the app.\n\n\
What you draw runs in a sealed frame: no network, no access to this app, nothing it renders can send \
anything anywhere. So it cannot fetch data, load an image from a URL, or use a library — write plain \
HTML and CSS, inline, with the values already in it. Inline `<script>` runs, and is for making the \
view react to a click, not for fetching anything.\n\n\
It is drawn on the app's own paper, so use the CSS variables that are already there — `var(--ink)`, \
`var(--muted)`, `var(--line)`, `var(--panel)`, `var(--accent)` — rather than colours of your own, and \
keep it to the width it is given. Never draw a field that asks for a password, a card number or any \
other credential: nothing you draw can receive one, so a box that asks for one is a trap even when \
you did not mean it as one.";

/// Why this view cannot be drawn, or `None` if it can.
///
/// The checks are about what the frame cannot enforce for us — size, and asking for
/// credentials — rather than about the HTML being well formed. A frame that renders
/// something odd is a bad view; a frame that asks for a password is a trap.
pub fn problem(title: &str, html: &str) -> Option<String> {
    if title.trim().is_empty() {
        return Some("Give the view a title — it is what the person sees above it.".into());
    }
    let body = html.trim();
    if body.is_empty() {
        return Some("There is nothing to draw: `html` is empty.".into());
    }
    if body.len() > MAX_BYTES {
        return Some(format!(
            "That view is {} KB, and the limit is {} KB. A view that big is a file — write it to one and say where it is.",
            body.len() / 1024,
            MAX_BYTES / 1024
        ));
    }
    if asks_for_a_credential(body) {
        return Some(
            "That view asks for a password or a card number. Nothing you draw can receive one — the frame cannot send \
             anything anywhere — so a box asking for one can only mislead. Take it out; if the person needs to type a \
             secret somewhere, say where."
                .into(),
        );
    }
    None
}

/// Does this draw a box that asks for a credential?
///
/// Read off the markup rather than the prose: a view may TALK about passwords (a security
/// report does), and what is refused is a field that looks like it takes one.
fn asks_for_a_credential(html: &str) -> bool {
    let lower = html.to_lowercase();
    let fields: Vec<&str> = lower.match_indices("<input").map(|(i, _)| &lower[i..(i + 400).min(lower.len())]).collect();
    fields.iter().any(|field| {
        let tag = field.split('>').next().unwrap_or(field);
        tag.contains("type=\"password\"")
            || tag.contains("type='password'")
            || ["password", "passcode", "card number", "cardnumber", "cvv", "security code", "api key", "secret"]
                .iter()
                .any(|word| tag.contains(word))
    })
}

/// The document as it is rendered: our policy, then the model's markup, then one line that
/// tells the app how tall it came out.
///
/// Built here rather than in the GUI so the policy travels with the view — anything that
/// stores or replays one gets the same sealed document, not a bare fragment somebody has
/// to remember to wrap.
///
/// The height line exists because the frame is its own origin: nothing outside can measure
/// it, but it can say. The app takes a number from that frame and no other, and clamps it,
/// so a view that lies about its height only changes its own size.
pub fn document(html: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\">\
         <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src data:; font-src data:; \
         style-src 'unsafe-inline'; script-src 'unsafe-inline'; base-uri 'none'; form-action 'none'\">\
         <style>:root{{color-scheme:light dark}}html,body{{margin:0;padding:0;background:transparent;\
         color:var(--ink,#1a1a1a);font:14px/1.5 -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif}}\
         *{{box-sizing:border-box;max-width:100%}}</style></head><body>{html}{HEIGHT_REPORTER}</body></html>"
    )
}

/// Tells the app the document's height whenever it changes. Mirrored in `GeneratedView.tsx`.
pub const HEIGHT_REPORTER: &str = "<script>(function(){var last=0;function send(){\
var h=Math.ceil(document.documentElement.scrollHeight);if(h!==last){last=h;\
parent.postMessage({superuserViewHeight:h},'*');}}addEventListener('load',send);\
if(window.ResizeObserver){new ResizeObserver(send).observe(document.documentElement);}\
setTimeout(send,0);})();</script>";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_view_says_its_height_and_nothing_else() {
        let doc = document("<p>14 months</p>");
        // After the markup, so a view's own script can't be the last word on its size.
        assert!(doc.ends_with(&format!("<p>14 months</p>{HEIGHT_REPORTER}</body></html>")));
        assert!(HEIGHT_REPORTER.contains("superuserViewHeight") && HEIGHT_REPORTER.contains("scrollHeight"));
        // A number is all it sends: no storage, no network, nothing about the page.
        for never in ["fetch", "XMLHttpRequest", "localStorage", "cookie", "innerHTML", "location"] {
            assert!(!HEIGHT_REPORTER.contains(never), "{never}");
        }
    }

    #[test]
    fn a_view_has_to_be_drawable_and_small() {
        assert_eq!(problem("Runway", "<p>14 months</p>"), None);
        assert!(problem("  ", "<p>x</p>").unwrap().contains("title"));
        assert!(problem("Runway", "   ").unwrap().contains("nothing to draw"));
        let huge = format!("<p>{}</p>", "x".repeat(MAX_BYTES));
        assert!(problem("Runway", &huge).unwrap().contains("is a file"));
    }

    #[test]
    fn a_box_that_asks_for_a_credential_is_refused_and_prose_about_one_is_not() {
        assert!(problem("Sign in", "<input type=\"password\" name=\"p\">").unwrap().contains("asks for a password"));
        assert!(problem("Sign in", "<input name='api key'>").is_some());
        assert!(problem("Card", "<input placeholder=\"Card number\">").is_some());
        // A report that TALKS about passwords is a report.
        assert_eq!(problem("Scan", "<p>Two passwords were found in the repo. Rotate them.</p>"), None);
        // An ordinary field is fine: it can't send anywhere either, and a filter that
        // refused every input would forbid half of what this is for.
        assert_eq!(problem("Notes", "<input placeholder=\"Try a number\">"), None);
    }

    #[test]
    fn the_document_seals_the_view_it_wraps() {
        let doc = document("<p>hello</p>");
        assert!(doc.contains("default-src 'none'"), "nothing it renders can fetch");
        assert!(doc.contains("form-action 'none'") && doc.contains("base-uri 'none'"));
        assert!(doc.contains("<p>hello</p>"));
        // The guidance promises what the sandbox enforces; if one moves, the other has to.
        assert!(GUIDANCE.contains("no network"), "the guidance must say what the policy enforces");
        assert!(GUIDANCE.contains("Never draw a field that asks for a password"), "and warn about the one thing we refuse");
    }
}
