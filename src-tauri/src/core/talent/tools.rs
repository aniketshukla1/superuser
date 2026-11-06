//! The Talent Scout's two tools, as the model reads them.

use super::*;
use crate::core::settings::Settings;

/// The candidates in a call: `[{name, email or emails, profile or profiles}]`.
fn candidates(args: &Value) -> Result<Vec<Person>, String> {
    let list = args["candidates"].as_array().filter(|a| !a.is_empty()).ok_or("give `candidates`: a list of {name, email, profiles}")?;
    if list.len() > 100 {
        return Err("at most 100 candidates a call".into());
    }
    list.iter()
        .enumerate()
        .map(|(i, c)| {
            let many = |one: &str, all: &str| {
                let mut v = split_values(c[one].as_str().unwrap_or(""));
                match c[all].as_array() {
                    Some(a) => v.extend(a.iter().filter_map(Value::as_str).flat_map(split_values)),
                    None => v.extend(split_values(c[all].as_str().unwrap_or(""))),
                }
                v
            };
            let p = Person { name: c["name"].as_str().unwrap_or("").trim().into(), emails: many("email", "emails"), profiles: many("profile", "profiles") };
            if p.name.is_empty() && p.emails.is_empty() && p.profiles.is_empty() {
                Err(format!("candidate {} has no name, email or profile", i + 1))
            } else {
                Ok(p)
            }
        })
        .collect()
}

/// Where a candidate already is.
enum Known<'a> {
    Applicant(usize, &'a Person, By),
    Shortlisted(&'a Entry, By),
}

/// The strongest match for `p`; at equal strength, the ATS before the shortlist.
fn known<'a>(p: &Person, export: &'a [(usize, Person)], shortlist: &'a [Entry]) -> Option<Known<'a>> {
    let applicants = export.iter().filter_map(|(row, q)| p.same_as(q).map(|by| Known::Applicant(*row, q, by)));
    let listed = shortlist.iter().filter_map(|e| p.same_as(&e.person).map(|by| Known::Shortlisted(e, by)));
    applicants.chain(listed).min_by_key(|k| match k {
        Known::Applicant(_, _, by) => (*by, 0),
        Known::Shortlisted(_, by) => (*by, 1),
    })
}

/// The ATS export named in Settings, or none set. `Err` is said to the model whole: an export
/// that can't be read means nobody was checked against it.
fn export(settings: &Settings) -> Result<Option<Vec<(usize, Person)>>, String> {
    let path = settings.talent_applicants.trim();
    if path.is_empty() {
        return Ok(None);
    }
    load_export(path).map(Some).map_err(|e| {
        format!("error: {e}. Nothing was checked against your applicants — say so, and ask the person to export it again (Settings ▸ General ▸ Talent Scout).")
    })
}

/// `candidates_check`: which candidates are already known — leave out, look twice, or new.
pub fn check_tool(data_dir: &Path, settings: &Settings, args: &Value) -> String {
    let people = match candidates(args) {
        Ok(p) => p,
        Err(e) => return format!("error: {e}"),
    };
    let export = match export(settings) {
        Ok(x) => x,
        Err(e) => return e,
    };
    let shortlist = load_shortlist(data_dir);
    let mut out = match &export {
        Some(x) => format!("Checked {} against your ATS export ({} people) and the {} shortlisted here.", people.len(), x.len(), shortlist.len()),
        None => format!(
            "Checked {} against the {} shortlisted here. No ATS export is set, so none were checked against your applicants — \
             say so in your results; the person sets one in Settings ▸ General ▸ Talent Scout.",
            people.len(),
            shortlist.len()
        ),
    };
    let rows = export.as_deref().unwrap_or_default();
    let (mut leave, mut look, mut new) = (vec![], vec![], vec![]);
    for p in &people {
        match known(p, rows, &shortlist) {
            Some(Known::Applicant(row, _, by)) if by != By::Name => leave.push(format!("- {} — in your ATS export, row {row} ({})", p.label(), by.say())),
            Some(Known::Shortlisted(e, by)) if by != By::Name => {
                leave.push(format!("- {} — shortlisted {} for “{}” ({})", p.label(), e.date, e.role, by.say()))
            }
            Some(Known::Applicant(row, q, _)) => look.push(format!("- {} — “{}” is in your ATS export, row {row}; no email or profile matched", p.label(), q.name)),
            Some(Known::Shortlisted(e, _)) => {
                look.push(format!("- {} — “{}” was shortlisted {} for “{}”; no email or profile matched", p.label(), e.person.name, e.date, e.role))
            }
            None => new.push(p.label()),
        }
    }
    if !leave.is_empty() {
        out.push_str(&format!("\n\nAlready known — leave them out ({}):\n{}", leave.len(), leave.join("\n")));
    }
    if !look.is_empty() {
        out.push_str(&format!("\n\nSame name only — check it's someone else before including ({}):\n{}", look.len(), look.join("\n")));
    }
    out.push_str(&format!("\n\nNew ({}): {}", new.len(), if new.is_empty() { "none".into() } else { new.join(", ") }));
    out.push_str(&match settings.talent_voice.trim() {
        "" => "\n\nNo sample of the person's own outreach is set: keep drafts short and plain, and say that a sample in Settings ▸ General ▸ Talent Scout would make them sound like them.".into(),
        v => format!("\n\nDraft outreach in the person's voice. A message they've sent:\n«{v}»"),
    });
    out
}

/// `candidates_shortlist`: keep the final list, refusing anyone already known.
pub fn shortlist_tool(data_dir: &Path, settings: &Settings, args: &Value, today: &str) -> String {
    let role = args["role"].as_str().map(str::trim).unwrap_or("");
    if role.is_empty() {
        return "error: give `role` — the role these candidates are for".into();
    }
    let people = match candidates(args) {
        Ok(p) => p,
        Err(e) => return format!("error: {e}"),
    };
    let export = match export(settings) {
        Ok(x) => x.unwrap_or_default(),
        Err(e) => return e,
    };
    let shortlist = load_shortlist(data_dir);
    let (mut keep, mut refused): (Vec<Entry>, Vec<String>) = (vec![], vec![]);
    for p in people {
        let twice = keep.iter().any(|e| matches!(p.same_as(&e.person), Some(By::Email | By::Profile)));
        let why = match known(&p, &export, &shortlist) {
            _ if !p.recognisable() => Some("no email or profile address, so they couldn't be recognised later".to_string()),
            _ if twice => Some("listed twice in this call".into()),
            Some(Known::Applicant(row, _, by)) if by != By::Name => Some(format!("already in your ATS export, row {row}")),
            Some(Known::Shortlisted(e, by)) if by != By::Name => Some(format!("already shortlisted {} for “{}”", e.date, e.role)),
            _ => None,
        };
        match why {
            Some(w) => refused.push(format!("- {} — {w}", p.label())),
            None => keep.push(Entry { date: today.into(), role: role.into(), person: p }),
        }
    }
    if !keep.is_empty() {
        if let Err(e) = append_shortlist(data_dir, &keep) {
            return format!("error: {e}");
        }
    }
    let mut out = format!("Shortlisted {} for “{role}” — later searches leave them out.", keep.len());
    if !refused.is_empty() {
        out.push_str(&format!("\n\nNot shortlisted ({}):\n{}", refused.len(), refused.join("\n")));
    }
    out
}
