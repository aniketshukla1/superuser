use super::super::*;

/// The Talent Scout's tools (`core::talent`). Neither contacts anyone: the check reads, and the
/// shortlist is the scout's own record of who it has put forward.
pub(in crate::core::agent) fn talent_tools() -> Vec<ToolSpec> {
    let candidates = json!({
        "type": "array",
        "description": "The candidates: each {name, email (or emails), profile (or profiles: GitHub, LinkedIn, a personal site)}.",
        "items": {"type": "object", "properties": {
            "name": {"type": "string"},
            "email": {"type": "string"},
            "profiles": {"type": "array", "items": {"type": "string"}}
        }}
    });
    vec![
        tool(
            "candidates_check",
            "Check candidates against the person's ATS export and everyone already shortlisted: who to leave out (same email or \
             profile), who shares only a name (check it's someone else), and who is new. Also gives the person's own outreach \
             message to write drafts in their voice. Call it before presenting anyone; leave out whoever it says to.",
            "talent",
            RiskClass::Read,
            params(json!({ "candidates": candidates }), &["candidates"]),
        ),
        tool(
            "candidates_shortlist",
            "Keep the final shortlist for a role, so later searches leave these people out. Refuses anyone already in the ATS \
             export or shortlisted, and anyone with no email or profile address to recognise them by. Contacts no one.",
            "talent",
            RiskClass::WriteLocal,
            params(json!({ "role": {"type": "string", "description": "The role they're shortlisted for."}, "candidates": candidates }), &["role", "candidates"]),
        ),
    ]
}
