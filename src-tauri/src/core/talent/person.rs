//! A person, and what recognises one across sources.

/// Someone as a source describes them. Any part may be empty.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Person {
    pub name: String,
    pub emails: Vec<String>,
    pub profiles: Vec<String>,
}

/// How two records turned out to be one person, strongest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum By {
    Email,
    Profile,
    /// Two people can share a name: said as a reason to look, never as proof.
    Name,
}

impl By {
    pub fn say(self) -> &'static str {
        match self {
            By::Email => "same email",
            By::Profile => "same profile",
            By::Name => "same name only",
        }
    }
}

/// Sites where the address alone names no one: `github.com` is everybody's.
const SHARED: &[&str] = &["github.com", "gitlab.com", "linkedin.com", "x.com", "twitter.com", "medium.com", "stackoverflow.com"];

/// An email as a key: lowercased, with a `+tag` dropped.
pub fn email_key(e: &str) -> Option<String> {
    let e = e.trim().trim_start_matches("mailto:").to_lowercase();
    let (local, domain) = e.split_once('@')?;
    let local = local.split('+').next().unwrap_or(local);
    (!local.is_empty() && domain.contains('.') && !domain.contains('@')).then(|| format!("{local}@{domain}"))
}

/// A profile address as a key: host and path, without the scheme, `www.`, query, fragment or
/// trailing slash — `linkedin.com/in/ada-lovelace`, `github.com/ada`. LinkedIn's country hosts
/// (`uk.linkedin.com`) are LinkedIn.
pub fn profile_key(u: &str) -> Option<String> {
    let u = u.trim().to_lowercase();
    let rest = u.split_once("://").map_or(u.as_str(), |(_, r)| r);
    let rest = rest.split(['?', '#']).next().unwrap_or("").trim_end_matches('/');
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = if host.ends_with(".linkedin.com") { "linkedin.com" } else { host };
    if !host.contains('.') || host.contains('@') || (path.is_empty() && SHARED.contains(&host)) {
        return None;
    }
    Some(if path.is_empty() { host.to_string() } else { format!("{host}/{path}") })
}

/// A name as a key: lowercased letters and digits, one space between words. One word isn't
/// enough to recognise anyone by.
pub fn name_key(n: &str) -> Option<String> {
    let k: String = n.chars().flat_map(|c| if c.is_alphanumeric() { c.to_lowercase().collect::<Vec<_>>() } else { vec![' '] }).collect();
    let k = k.split_whitespace().collect::<Vec<_>>().join(" ");
    k.contains(' ').then_some(k)
}

impl Person {
    /// How `self` and `other` are one person, if they are: the strongest way that holds.
    pub fn same_as(&self, other: &Person) -> Option<By> {
        let shared = |a: &[String], b: &[String], key: fn(&str) -> Option<String>| {
            a.iter().filter_map(|x| key(x)).any(|k| b.iter().filter_map(|y| key(y)).any(|j| j == k))
        };
        if shared(&self.emails, &other.emails, email_key) {
            return Some(By::Email);
        }
        if shared(&self.profiles, &other.profiles, profile_key) {
            return Some(By::Profile);
        }
        match (name_key(&self.name), name_key(&other.name)) {
            (Some(a), Some(b)) if a == b => Some(By::Name),
            _ => None,
        }
    }

    /// Whether anything but a name would recognise this person later.
    pub fn recognisable(&self) -> bool {
        self.emails.iter().any(|e| email_key(e).is_some()) || self.profiles.iter().any(|p| profile_key(p).is_some())
    }

    /// The name, or the first thing that identifies them when there's no name.
    pub fn label(&self) -> String {
        [Some(self.name.trim()), self.emails.first().map(|s| s.trim()), self.profiles.first().map(|s| s.trim())]
            .into_iter()
            .flatten()
            .find(|s| !s.is_empty())
            .unwrap_or("(no name)")
            .to_string()
    }
}

/// Several values in one cell or argument: split on commas, semicolons, pipes and whitespace.
pub fn split_values(s: &str) -> Vec<String> {
    s.split([',', ';', '|', ' ', '\n', '\t']).map(str::trim).filter(|x| !x.is_empty()).map(String::from).collect()
}
