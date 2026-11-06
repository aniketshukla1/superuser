use super::*;

/// Talent Scout's route takes its two settings and nothing else, refuses an export it can't read,
/// and says how many people the export and the shortlist hold.
#[test]
fn the_talent_route_refuses_an_export_it_cannot_read_and_counts_the_rest() {
    let dir = std::env::temp_dir().join(format!("su-talent-api-{}", crate::core::inbox::short_id()));
    std::fs::create_dir_all(&dir).unwrap();
    let export = dir.join("ats.csv");
    std::fs::write(&export, "Name,Email\nAda Lovelace,ada@example.com\nGrace Hopper,grace@example.com\n").unwrap();
    let path = export.to_string_lossy().into_owned();

    let ok = talent_patch(&json!({ "talent_applicants": path, "talent_voice": "Hi —", "perf_signals": "x" })).unwrap();
    assert_eq!(ok, json!({ "talent_applicants": path, "talent_voice": "Hi —" }), "only its own two");
    let err = talent_patch(&json!({ "talent_applicants": dir.join("nope.csv").to_string_lossy() })).unwrap_err();
    assert!(err.starts_with("Couldn't read your ATS export") && err.ends_with('.'), "{err}");
    assert_eq!(talent_patch(&json!({ "talent_applicants": "" })), Ok(json!({ "talent_applicants": "" })), "clearing it is allowed");

    let mut settings = crate::core::settings::Settings::default();
    settings.apply_patch(&ok);
    let status = talent_status(&settings, &dir);
    assert_eq!((status["people"].clone(), status["error"].clone(), status["shortlisted"].clone()), (json!(2), Value::Null, json!(0)));
    std::fs::remove_file(&export).unwrap();
    assert!(talent_status(&settings, &dir)["error"].as_str().unwrap().starts_with("Couldn't read"), "an export moved away is said");
    let _ = std::fs::remove_dir_all(&dir);
}
