use super::*;
use serde_json::json;

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("su-talent-{tag}-{}", crate::core::inbox::short_id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn people_are_recognised_by_email_then_profile_then_name() {
    assert_eq!(email_key(" Ada+jobs@Example.COM "), Some("ada@example.com".into()));
    assert_eq!(email_key("mailto:ada@example.com"), Some("ada@example.com".into()));
    assert_eq!(email_key("not an email"), None);
    assert_eq!(profile_key("https://uk.linkedin.com/in/ada-lovelace/?trk=x"), Some("linkedin.com/in/ada-lovelace".into()));
    assert_eq!(profile_key("www.github.com/Ada/"), Some("github.com/ada".into()));
    assert_eq!(profile_key("https://ada.dev"), Some("ada.dev".into()), "a personal site is a profile");
    assert_eq!(profile_key("https://github.com/"), None, "everybody's GitHub isn't anybody's");
    assert_eq!(name_key("  Ada  LOVELACE, PhD "), Some("ada lovelace phd".into()));
    assert_eq!(name_key("Ada"), None, "one word recognises nobody");

    let ada = Person { name: "Ada Lovelace".into(), emails: vec!["ada@example.com".into()], profiles: vec!["github.com/ada".into()] };
    let same_email = Person { name: "A. Lovelace".into(), emails: vec!["ADA+x@example.com".into()], ..Default::default() };
    let same_profile = Person { profiles: vec!["https://github.com/ada".into()], ..Default::default() };
    let same_name = Person { name: "ada lovelace".into(), emails: vec!["other@example.com".into()], ..Default::default() };
    assert_eq!(ada.same_as(&same_email), Some(By::Email));
    assert_eq!(ada.same_as(&same_profile), Some(By::Profile));
    assert_eq!(ada.same_as(&same_name), Some(By::Name));
    assert_eq!(ada.same_as(&Person { name: "Grace Hopper".into(), ..Default::default() }), None);
    assert!(same_name.recognisable(), "an email recognises them");
    assert!(!Person { name: "Ada Lovelace".into(), ..Default::default() }.recognisable());
}

#[test]
fn an_export_is_read_whatever_its_ats_calls_the_columns() {
    // Greenhouse-ish: split names, several emails in one cell, a BOM on the first header.
    let gh = "\u{feff}First Name,Last Name,Email Addresses,LinkedIn URL,Stage\nAda,Lovelace,\"ada@example.com, ada@work.io\",linkedin.com/in/ada,Offer\n,,,,\nGrace,Hopper,,,Applied\n";
    let people = read_export(gh).unwrap();
    assert_eq!(people.len(), 2, "an empty row is no one");
    assert_eq!((people[0].0, people[0].1.name.as_str()), (2, "Ada Lovelace"), "row as a spreadsheet numbers it");
    assert_eq!(people[0].1.emails, vec!["ada@example.com", "ada@work.io"]);
    assert_eq!(people[0].1.profiles, vec!["linkedin.com/in/ada"]);
    assert_eq!(people[1].0, 4);
    // Lever-ish: one name column, a links column.
    let lever = "Name,Email,Links\nLinus Torvalds,linus@example.org,https://github.com/torvalds https://lkml.org\n";
    assert_eq!(read_export(lever).unwrap()[0].1.profiles.len(), 2);
    assert!(read_export("Stage,Owner\nOffer,Sam\n").unwrap_err().contains("no name, email or profile column"));
    assert!(read_export("").is_err());
}

#[test]
fn the_shortlist_keeps_only_what_recognises_people_and_can_be_forgotten() {
    let dir = scratch("ledger");
    let ada = Person { name: "Ada, \"the Countess\"".into(), emails: vec!["ada@example.com".into()], profiles: vec!["github.com/ada".into()] };
    append_shortlist(&dir, &[Entry { date: "2026-11-03".into(), role: "Staff SRE".into(), person: ada.clone() }]).unwrap();
    let back = load_shortlist(&dir);
    assert_eq!((back.len(), back[0].person.clone(), back[0].role.as_str()), (1, ada, "Staff SRE"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(shortlist_path(&dir)).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "other people's details are this account's alone: {mode:o}");
    }
    assert_eq!(forget_shortlist(&dir), Ok(1));
    assert!(load_shortlist(&dir).is_empty());
    assert_eq!(forget_shortlist(&dir), Ok(0));
    let _ = std::fs::remove_dir_all(&dir);
}

fn settings_with(export: &Path, voice: &str) -> crate::core::settings::Settings {
    let mut s = crate::core::settings::Settings::default();
    s.talent_applicants = export.to_string_lossy().into_owned();
    s.talent_voice = voice.into();
    s
}

#[test]
fn the_check_says_who_to_leave_out_who_to_look_at_twice_and_who_is_new() {
    let dir = scratch("check");
    let export = dir.join("ats.csv");
    std::fs::write(&export, "Name,Email,LinkedIn\nAda Lovelace,ada@example.com,\nAlan Turing,alan@old.ac.uk,\n").unwrap();
    let grace = Person { name: "Grace Hopper".into(), profiles: vec!["github.com/grace".into()], ..Default::default() };
    append_shortlist(&dir, &[Entry { date: "2026-10-20".into(), role: "Compiler engineer".into(), person: grace }]).unwrap();
    let args = json!({ "candidates": [
        { "name": "Ada L.", "email": "ADA@example.com" },
        { "name": "G. Hopper", "profiles": ["https://github.com/grace/"] },
        { "name": "Alan Turing", "profile": "https://github.com/aturing" },
        { "name": "Linus Torvalds", "profile": "github.com/torvalds" },
    ]});
    let out = check_tool(&dir, &settings_with(&export, "Hi Sam — saw your talk on…"), &args);
    assert!(out.starts_with("Checked 4 against your ATS export (2 people) and the 1 shortlisted here."), "{out}");
    assert!(out.contains("- Ada L. — in your ATS export, row 2 (same email)"), "{out}");
    assert!(out.contains("- G. Hopper — shortlisted 2026-10-20 for “Compiler engineer” (same profile)"), "{out}");
    assert!(out.contains("check it's someone else before including (1):\n- Alan Turing — “Alan Turing” is in your ATS export, row 3"), "{out}");
    assert!(out.contains("New (1): Linus Torvalds"), "{out}");
    assert!(out.ends_with("A message they've sent:\n«Hi Sam — saw your talk on…»"), "{out}");

    let unset = check_tool(&dir, &crate::core::settings::Settings::default(), &args);
    assert!(unset.contains("No ATS export is set") && unset.contains("No sample of the person's own outreach"), "{unset}");
    let missing = check_tool(&dir, &settings_with(&dir.join("gone.csv"), ""), &args);
    assert!(missing.starts_with("error: couldn't read your ATS export") && missing.contains("Nothing was checked"), "{missing}");
    assert!(check_tool(&dir, &settings_with(&export, ""), &json!({ "candidates": [{}] })).contains("candidate 1 has no name"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_shortlist_refuses_applicants_repeats_and_people_it_could_not_recognise_again() {
    let dir = scratch("shortlist");
    let export = dir.join("ats.csv");
    std::fs::write(&export, "Email\nada@example.com\n").unwrap();
    let s = settings_with(&export, "");
    let args = json!({ "role": "Staff SRE", "candidates": [
        { "name": "Ada Lovelace", "email": "ada@example.com" },
        { "name": "Linus Torvalds", "profile": "github.com/torvalds" },
        { "name": "Linus T.", "profile": "https://github.com/torvalds/" },
        { "name": "Nameless Wonder" },
    ]});
    let out = shortlist_tool(&dir, &s, &args, "2026-11-03");
    assert!(out.starts_with("Shortlisted 1 for “Staff SRE”"), "{out}");
    assert!(out.contains("- Ada Lovelace — already in your ATS export, row 2"), "{out}");
    assert!(out.contains("- Linus T. — listed twice in this call"), "{out}");
    assert!(out.contains("- Nameless Wonder — no email or profile address"), "{out}");
    let again = shortlist_tool(&dir, &s, &json!({ "role": "SRE II", "candidates": [{ "name": "Linus", "profile": "github.com/torvalds" }] }), "2026-11-04");
    assert!(again.starts_with("Shortlisted 0") && again.contains("already shortlisted 2026-11-03 for “Staff SRE”"), "{again}");
    assert!(shortlist_tool(&dir, &s, &json!({ "candidates": [{ "name": "X Y", "email": "x@y.io" }] }), "2026-11-03").contains("give `role`"));
    let _ = std::fs::remove_dir_all(&dir);
}
