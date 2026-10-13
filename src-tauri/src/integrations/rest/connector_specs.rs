//! the connector specs

use super::*;

// Everything whose API is "attach a credential, GET a list" lives here as data.
// OAuth-only connectors (Dropbox, Box, Canva, Docusign, QuickBooks, Salesforce,
// Outlook, Drive), socket/gateway ones (Slack, Discord, WhatsApp), IMAP email and
// the browser keep their own path and are not listed here.

pub(super) const fn feed(path: &'static str, items_at: &'static str, id_key: &'static str, title_keys: &'static [&'static str], body_keys: &'static [(&'static str, &'static str)], kind: &'static str) -> Feed {
    Feed { path, items_at, id_key, title_keys, body_keys, kind, post_body: None }
}

pub const SPECS: &[RestSpec] = &[
    RestSpec { id: "asana", name: "Asana", base: "https://app.asana.com/api/1.0", auth: AuthStyle::Bearer, verify_path: "/users/me", identity_at: "data.name", extra_headers: &[],
        feeds: &[feed("/users/me/favorites?resource_type=project", "data", "gid", &["name"], &[("Resource", "resource_type")], "project")] },
    RestSpec { id: "clickup", name: "ClickUp", base: "https://api.clickup.com/api/v2", auth: AuthStyle::Raw, verify_path: "/user", identity_at: "user.username", extra_headers: &[],
        feeds: &[feed("/team", "teams", "id", &["name"], &[("Color", "color")], "team")] },
    RestSpec { id: "jira", name: "Jira", base: "https://{site}/rest/api/3", auth: AuthStyle::BasicEmailToken, verify_path: "/myself", identity_at: "displayName", extra_headers: &[],
        feeds: &[feed("/search?jql=assignee%3DcurrentUser()%20ORDER%20BY%20updated%20DESC&maxResults=25", "issues", "key", &["key"], &[("Summary", "fields"), ("Status", "fields")], "issue")] },
    RestSpec { id: "confluence", name: "Confluence", base: "https://{site}/wiki/rest/api", auth: AuthStyle::BasicEmailToken, verify_path: "/user/current", identity_at: "displayName", extra_headers: &[],
        feeds: &[feed("/content?limit=25", "results", "id", &["title"], &[("Type", "type"), ("Status", "status")], "page")] },
    RestSpec { id: "zendesk", name: "Zendesk", base: "https://{site}.zendesk.com/api/v2", auth: AuthStyle::BasicEmailToken, verify_path: "/users/me.json", identity_at: "user.name", extra_headers: &[],
        feeds: &[feed("/tickets.json?per_page=25&sort_order=desc", "tickets", "id", &["subject"], &[("Status", "status"), ("Priority", "priority"), ("Description", "description")], "ticket")] },
    RestSpec { id: "gitlab", name: "GitLab", base: "https://gitlab.com/api/v4", auth: AuthStyle::Bearer, verify_path: "/user", identity_at: "name", extra_headers: &[],
        feeds: &[feed("/issues?scope=assigned_to_me&per_page=25", "", "id", &["title"], &[("State", "state"), ("Description", "description"), ("Web", "web_url")], "issue")] },
    RestSpec { id: "pagerduty", name: "PagerDuty", base: "https://api.pagerduty.com", auth: AuthStyle::Header("Authorization", "Token token="), verify_path: "/users/me", identity_at: "user.name", extra_headers: &[],
        feeds: &[feed("/incidents?limit=25&sort_by=created_at:desc", "incidents", "id", &["title"], &[("Status", "status"), ("Urgency", "urgency"), ("Service", "service")], "incident")] },
    RestSpec { id: "datadog", name: "Datadog", base: "https://api.datadoghq.com/api/v1", auth: AuthStyle::Header("DD-API-KEY", ""), verify_path: "/validate", identity_at: "valid", extra_headers: &[("DD-APPLICATION-KEY", "app_key")],
        feeds: &[feed("/monitor", "", "id", &["name"], &[("Status", "overall_state"), ("Type", "type"), ("Query", "query")], "monitor")] },
    RestSpec { id: "figma", name: "Figma", base: "https://api.figma.com/v1", auth: AuthStyle::Header("X-Figma-Token", ""), verify_path: "/me", identity_at: "handle", extra_headers: &[], feeds: &[] },
    RestSpec { id: "posthog", name: "PostHog", base: "https://app.posthog.com/api", auth: AuthStyle::Bearer, verify_path: "/users/@me/", identity_at: "email", extra_headers: &[],
        feeds: &[feed("/projects/@current/insights/?limit=20", "results", "id", &["name", "derived_name"], &[("Description", "description")], "insight")] },
    RestSpec { id: "hubspot", name: "HubSpot", base: "https://api.hubapi.com", auth: AuthStyle::Bearer, verify_path: "/account-info/v3/details", identity_at: "portalId", extra_headers: &[],
        feeds: &[feed("/crm/v3/objects/deals?limit=25", "results", "id", &["properties"], &[("Properties", "properties")], "deal")] },
    RestSpec { id: "attio", name: "Attio", base: "https://api.attio.com/v2", auth: AuthStyle::Bearer, verify_path: "/self", identity_at: "workspace_name", extra_headers: &[], feeds: &[] },
    RestSpec { id: "close", name: "Close", base: "https://api.close.com/api/v1", auth: AuthStyle::Bearer, verify_path: "/me/", identity_at: "email", extra_headers: &[],
        feeds: &[feed("/lead/?_limit=25", "data", "id", &["display_name", "name"], &[("Status", "status_label"), ("Description", "description")], "lead")] },
    RestSpec { id: "apollo", name: "Apollo.io", base: "https://api.apollo.io/v1", auth: AuthStyle::Header("X-Api-Key", ""), verify_path: "/auth/health", identity_at: "is_logged_in", extra_headers: &[], feeds: &[] },
    RestSpec { id: "hunter", name: "Hunter", base: "https://api.hunter.io/v2", auth: AuthStyle::Query("api_key"), verify_path: "/account", identity_at: "data.email", extra_headers: &[], feeds: &[] },
    RestSpec { id: "mixpanel", name: "Mixpanel", base: "https://mixpanel.com/api/app", auth: AuthStyle::BasicEmailToken, verify_path: "/me", identity_at: "results.email", extra_headers: &[], feeds: &[] },
    RestSpec { id: "discord", name: "Discord", base: "https://discord.com/api/v10", auth: AuthStyle::Header("Authorization", "Bot "), verify_path: "/users/@me", identity_at: "username", extra_headers: &[],
        feeds: &[feed("/users/@me/guilds", "", "id", &["name"], &[("Owner", "owner")], "server")] },
    RestSpec { id: "amplitude", name: "Amplitude", base: "https://amplitude.com/api/2", auth: AuthStyle::BasicEmailToken, verify_path: "/sessions/average", identity_at: "", extra_headers: &[], feeds: &[] },
];

pub fn find_spec(id: &str) -> Option<&'static RestSpec> {
    SPECS.iter().find(|s| s.id == id)
}
