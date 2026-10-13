use super::*;

/// The floor itself, for whichever kind of remote end is being set up. Same lines either
/// way — the reasons above are about the address, not about what answers at it.
pub fn endpoint_problem(url: &str, what: &Addressee) -> Option<String> {
    let raw = url.trim();
    if raw.is_empty() {
        return Some(format!("give the {}'s address", what.noun));
    }
    let Some(rest) = raw.strip_prefix("https://") else {
        return Some(if raw.starts_with("http://") {
            format!(
                "that address is plain http. {} has to be https — otherwise every call it carries is readable by \
                 anything on the way.",
                what.subject
            )
        } else {
            "that address has to start with https://".to_string()
        });
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return Some(format!(
            "that address carries a username or password in it. Those end up in the config file, in the trail and in \
             every error that quotes the address — put the credential in {} instead.",
            what.credential_home
        ));
    }
    let host = authority.rsplit_once(':').map(|(h, _)| h).unwrap_or(authority).trim_end_matches('.').to_lowercase();
    if host.is_empty() {
        return Some("that address has no host in it".into());
    }
    // The query and fragment, for anything that looks like a credential being passed along.
    let tail = &raw[raw.len().min("https://".len() + authority.len())..];
    let lower_tail = tail.to_lowercase();
    for word in ["token=", "api_key=", "apikey=", "key=", "secret=", "password=", "access_token=", "auth="] {
        if lower_tail.contains(word) {
            return Some(format!(
                "that address carries `{}` in it. A credential in a URL is a credential in every log that URL reaches — \
                 {} is where it belongs.",
                word.trim_end_matches('='),
                what.credential_home
            ));
        }
    }
    // An IP literal is refused as one whatever it points at: the reason a person cannot
    // check it is the address itself, and the "this machine" wording would be wrong for a
    // public address.
    if host.parse::<std::net::IpAddr>().is_ok() {
        return Some(format!(
            "that address is a bare IP. Use the {}'s name, so what it is can be checked — and so a certificate \
             means something.",
            what.noun
        ));
    }
    // Named before the general floor, because these say WHY in words a person can act on:
    // "that is a name inside your own network" beats "that is blocked".
    for private in [".local", ".internal", ".localdomain", ".svc", ".home.arpa"] {
        if host.ends_with(private) {
            return Some(format!(
                "`{private}` names something inside a private network rather than {} on the internet.",
                what.kind
            ));
        }
    }
    if crate::core::webguard::is_blocked_host(&host) {
        return Some(format!(
            "that address points at this machine or its own network. {} is somewhere else; this would reach services \
             that are not on the internet at all.",
            capitalised(what.kind)
        ));
    }
    if !host.contains('.') {
        return Some(format!("that address has no domain in it — a name like `{}` is what this needs.", what.example));
    }
    None
}

pub(super) fn capitalised(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

/// Whether this failure means the app has taken our access away, rather than that the call
/// went wrong.
///
/// Worth telling apart because the fix is different and only one of them is the person's:
/// a network blip is worth retrying, and a revoked token will fail identically forever
/// until somebody signs in again. A run that retries the second looks broken for reasons
/// nobody can see.
pub fn is_revoked(error: &str) -> bool {
    let e = error.to_lowercase();
    ["401", "unauthorized", "invalid_token", "invalid_grant", "token expired", "token has expired", "revoked", "reauthenticate", "re-authenticate"]
        .iter()
        .any(|needle| e.contains(needle))
}

/// What to say when an app has stopped accepting our sign-in.
pub fn revoked(server: &str) -> String {
    format!(
        "{server} is no longer accepting this app's sign-in — the access was revoked or it expired. Nothing was done. \
         Tell the person plainly: they reconnect {server} under Connected apps, and then this will work again. Don't \
         retry it this turn."
    )
}

/// How a connected app's answer is said back, so three different things never read alike.
///
/// The one that caused real harm is the middle: a vendor's own "the caller does not have
/// permission" was handed to the model as a plain result, and the model reported it as
/// "you don't have access to that" — a claim about the user's account, made up out of an
/// error the app returned.
pub fn vendor_error(text: &str) -> String {
    format!("The app reported an error: {}", one_line(text))
}

pub fn call_failed(why: &str) -> String {
    format!("That tool could not be called: {}", one_line(why))
}

/// What an empty answer means, said rather than left blank.
pub fn empty_result(server: &str, tool: &str) -> String {
    format!(
        "{server} ran `{tool}` and returned nothing at all. That is not the same as \"none\" — it may have worked and had \
         nothing to say, or matched nothing. Say which you think it is, or look another way; don't report a result."
    )
}

pub(super) fn one_line(text: &str) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    flat.chars().take(400).collect()
}
