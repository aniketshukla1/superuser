
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Auth {
    ApiToken,
    Token,
    BotToken,
    OAuth,
    AppPassword,
    None,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub secret: bool,
    pub placeholder: &'static str,
    /// The connect form marks the others "(optional)" and lets them be left blank.
    pub required: bool,
    /// A line under the field, when its label can't say enough.
    pub help: &'static str,
}

pub(super) const fn f(key: &'static str, label: &'static str, secret: bool, placeholder: &'static str) -> Field {
    Field { key, label, secret, placeholder, required: true, help: "" }
}

/// A field that may be left blank — one of several ways in, say.
pub(super) const fn opt(key: &'static str, label: &'static str, secret: bool, placeholder: &'static str, help: &'static str) -> Field {
    Field { key, label, secret, placeholder, required: false, help }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Connector {
    pub id: &'static str,
    pub title: &'static str,
    pub blurb: &'static str,
    pub auth: Auth,
    pub category: &'static str,
    pub brand_color: &'static str,
    /// Can receive inbound messages/events (not just be read).
    pub two_way: bool,
    /// Has a working client in this app today.
    pub implemented: bool,
    pub fields: &'static [Field],
    pub instructions: &'static [&'static str],
    /// Extra typeahead terms the title doesn't carry.
    pub aliases: &'static [&'static str],
}

pub(super) const TOKEN_F: &[Field] = &[f("token", "API token", true, "")];

pub(super) const KEY_F: &[Field] = &[f("api_key", "API key", true, "")];

pub const CONNECTORS: &[Connector] = &[
    // ---- messaging ----
    Connector { id: "slack", title: "Slack", blurb: "Read channels, post replies, and take @mentions as work.", auth: Auth::BotToken, category: "Messaging", brand_color: "#611f69", two_way: true, implemented: true,
        fields: &[f("bot_token", "Bot token", true, "xoxb-…"), f("app_token", "App token", true, "xapp-…")],
        instructions: &["Create an app at api.slack.com/apps", "Add bot scopes (channels:history, chat:write), install to the workspace", "Copy the Bot token (xoxb-…); for inbound events add an App token (xapp-…)"], aliases: &["chat", "messages"] },
    Connector { id: "telegram", title: "Telegram", blurb: "Send and receive messages through your own bot.", auth: Auth::BotToken, category: "Messaging", brand_color: "#229ed9", two_way: true, implemented: true,
        fields: &[f("bot_token", "Bot token", true, "123456:ABC-DEF…")],
        instructions: &["Message @BotFather → /newbot", "Copy the token it gives you", "Message @userinfobot to get your numeric chat id"], aliases: &["chat", "messages"] },
    Connector { id: "discord", title: "Discord", blurb: "Read and post in your servers.", auth: Auth::BotToken, category: "Messaging", brand_color: "#5865f2", two_way: false, implemented: true,
        fields: &[f("bot_token", "Bot token", true, "")],
        instructions: &["Create an app at discord.com/developers", "Add a bot, copy its token", "Invite the bot to your server"], aliases: &["chat"] },
    Connector { id: "whatsapp", title: "WhatsApp", blurb: "Send messages via the Cloud API.", auth: Auth::Token, category: "Messaging", brand_color: "#25d366", two_way: false, implemented: true,
        fields: &[f("token", "Access token", true, ""), f("phone_number_id", "Phone number id", false, "")],
        instructions: &["Set up WhatsApp Cloud API in Meta for Developers", "Copy the access token and phone number id"], aliases: &["chat", "messages"] },
    Connector { id: "email", title: "Email (IMAP)", blurb: "Read and send from any IMAP/SMTP mailbox.", auth: Auth::AppPassword, category: "Email", brand_color: "#6b7280", two_way: false, implemented: true,
        fields: &[f("address", "Email address", false, "you@example.com"), f("app_password", "App password", true, ""), f("imap_host", "IMAP host", false, "imap.example.com")],
        instructions: &["Enable IMAP in your mail provider", "Create an app password (not your login password)", "Paste the address, app password, and IMAP host"], aliases: &["mail", "imap", "smtp"] },

    // ---- google / microsoft ----
    Connector { id: "gmail", title: "Gmail", blurb: "Triage your inbox, summarize threads, draft replies.", auth: Auth::OAuth, category: "Email", brand_color: "#ea4335", two_way: false, implemented: true,
        fields: &[], instructions: &["Sign in with Google (one sign-in covers Gmail and Calendar)"], aliases: &["mail", "email"] },
    Connector { id: "calendar", title: "Google Calendar", blurb: "See what's ahead and find free time.", auth: Auth::OAuth, category: "Calendar", brand_color: "#4285f4", two_way: false, implemented: true,
        fields: &[], instructions: &["Sign in with Google (one sign-in covers Gmail and Calendar)"], aliases: &["meetings", "schedule", "events"] },
    Connector { id: "google_drive", title: "Google Drive", blurb: "Read and write documents in your Drive.", auth: Auth::OAuth, category: "Files", brand_color: "#4285f4", two_way: false, implemented: true,
        fields: &[], instructions: &["Sign in with Google and grant Drive access"], aliases: &["files", "docs", "sheets"] },
    Connector { id: "outlook", title: "Outlook", blurb: "Mail and calendar from Microsoft 365.", auth: Auth::OAuth, category: "Email", brand_color: "#0078d4", two_way: false, implemented: true,
        fields: &[], instructions: &["Register an app in Entra ID (Azure AD)", "Grant Mail.Read / Calendars.Read", "Sign in to authorize"], aliases: &["mail", "email", "calendar", "microsoft", "office"] },

    // ---- dev ----
    Connector { id: "github", title: "GitHub", blurb: "Issues, PRs and repo activity.", auth: Auth::Token, category: "Developer", brand_color: "#1f2328", two_way: true, implemented: true,
        fields: &[f("token", "Personal access token", true, "ghp_…")],
        instructions: &["Open github.com/settings/tokens", "Create a token with repo + read:user", "Paste it here — it's verified before saving"], aliases: &["git", "code", "issues", "prs"] },
    Connector { id: "gitlab", title: "GitLab", blurb: "Issues, merge requests and pipelines.", auth: Auth::Token, category: "Developer", brand_color: "#fc6d26", two_way: false, implemented: true,
        fields: &[f("token", "Personal access token", true, "glpat-…")],
        instructions: &["User Settings → Access Tokens", "Create a token with api scope"], aliases: &["git", "code"] },
    Connector { id: "jira", title: "Jira", blurb: "Track issues and sprints.", auth: Auth::ApiToken, category: "Project", brand_color: "#0052cc", two_way: false, implemented: true,
        fields: &[f("email", "Account email", false, ""), f("api_token", "API token", true, ""), f("site", "Site url", false, "yourteam.atlassian.net")],
        instructions: &["Create an API token at id.atlassian.com/manage/api-tokens", "Paste your email, the token, and your site url"], aliases: &["atlassian", "tickets", "issues"] },
    Connector { id: "confluence", title: "Confluence", blurb: "Read and write team docs.", auth: Auth::ApiToken, category: "Docs", brand_color: "#172b4d", two_way: false, implemented: true,
        fields: &[f("email", "Account email", false, ""), f("api_token", "API token", true, ""), f("site", "Site url", false, "yourteam.atlassian.net")],
        instructions: &["Create an API token at id.atlassian.com/manage/api-tokens", "Paste your email, the token, and your site url"], aliases: &["atlassian", "wiki", "docs"] },
    Connector { id: "linear", title: "Linear", blurb: "Issues and project state.", auth: Auth::ApiToken, category: "Project", brand_color: "#5e6ad2", two_way: false, implemented: true,
        fields: KEY_F, instructions: &["Linear → Settings → API", "Create a personal API key"], aliases: &["issues", "tickets"] },
    Connector { id: "pagerduty", title: "PagerDuty", blurb: "See who's on call and what's firing.", auth: Auth::ApiToken, category: "Ops", brand_color: "#06ac38", two_way: false, implemented: true,
        fields: KEY_F, instructions: &["PagerDuty → Integrations → API Access Keys", "Create a read-only key"], aliases: &["oncall", "incidents", "alerts"] },
    // Queried live by the Product Performance assistant, never copied into memory (`integrations::prometheus`).
    Connector { id: "prometheus", title: "Prometheus", blurb: "Query metrics live — latency, errors, saturation — to investigate a slowdown. Also Thanos, Mimir, VictoriaMetrics and Grafana Cloud.", auth: Auth::Token, category: "Ops", brand_color: "#e6522c", two_way: false, implemented: true,
        fields: &[
            f("url", "Address", false, "http://prometheus.internal:9090"),
            opt("username", "Username", false, "", "For basic auth. Grafana Cloud: your instance ID."),
            opt("password", "Password or API token", true, "", "For basic auth. Grafana Cloud: an access policy token that can read metrics."),
            opt("token", "Bearer token", true, "", "Instead of basic auth, when a proxy in front of Prometheus wants one."),
            opt("org_id", "Tenant", false, "", "Mimir or Cortex with several tenants: sent as X-Scope-OrgID."),
        ],
        instructions: &[
            "Paste the address you open Prometheus at. For Grafana Cloud, the Prometheus query endpoint (it ends /api/prom)",
            "Most Prometheus servers need nothing more. Behind a login, fill in basic auth or a bearer token",
            "Only read endpoints are ever called — queries, labels, alerts. Nothing is copied into memory",
        ],
        aliases: &["metrics", "monitoring", "observability", "promql", "thanos", "mimir", "victoriametrics", "grafana", "latency"] },
    Connector { id: "datadog", title: "Datadog", blurb: "Metrics, monitors and incident timelines.", auth: Auth::ApiToken, category: "Ops", brand_color: "#632ca6", two_way: false, implemented: true,
        fields: &[f("api_key", "API key", true, ""), f("app_key", "Application key", true, "")],
        instructions: &["Organization Settings → API Keys", "Also create an Application key"], aliases: &["metrics", "monitoring", "alerts", "observability"] },

    // ---- project / productivity ----
    Connector { id: "notion", title: "Notion", blurb: "Read and write your workspace pages.", auth: Auth::ApiToken, category: "Docs", brand_color: "#1f2328", two_way: false, implemented: true,
        fields: &[f("token", "Internal integration token", true, "ntn_… / secret_…")],
        instructions: &["Open notion.so/my-integrations", "Create an internal integration and copy its secret", "Share the pages you want indexed with the integration"], aliases: &["docs", "wiki", "notes"] },
    Connector { id: "asana", title: "Asana", blurb: "Tasks and projects.", auth: Auth::Token, category: "Project", brand_color: "#f06a6a", two_way: false, implemented: true,
        fields: TOKEN_F, instructions: &["Asana → My Settings → Apps → Developer apps", "Create a personal access token"], aliases: &["tasks", "projects"] },
    Connector { id: "clickup", title: "ClickUp", blurb: "Tasks, docs and goals.", auth: Auth::ApiToken, category: "Project", brand_color: "#7b68ee", two_way: false, implemented: true,
        fields: TOKEN_F, instructions: &["ClickUp → Settings → Apps", "Copy your personal API token"], aliases: &["tasks", "projects"] },
    Connector { id: "monday", title: "monday.com", blurb: "Boards, items and updates.", auth: Auth::ApiToken, category: "Project", brand_color: "#6161ff", two_way: true, implemented: false,
        fields: TOKEN_F, instructions: &["monday.com → Developers → My Access Tokens", "Copy the token"], aliases: &["boards", "tasks", "projects"] },

    // ---- crm / sales ----
    Connector { id: "hubspot", title: "HubSpot", blurb: "Contacts, deals and pipeline.", auth: Auth::Token, category: "CRM", brand_color: "#ff7a59", two_way: false, implemented: true,
        fields: TOKEN_F, instructions: &["Create a private app in HubSpot", "Copy its access token"], aliases: &["crm", "sales", "deals", "contacts"] },
    Connector { id: "salesforce", title: "Salesforce", blurb: "Accounts, opportunities and reports.", auth: Auth::OAuth, category: "CRM", brand_color: "#00a1e0", two_way: false, implemented: true,
        fields: &[], instructions: &["Create a Connected App in Salesforce Setup", "Authorize it to link your org"], aliases: &["crm", "sales"] },
    Connector { id: "attio", title: "Attio", blurb: "Modern CRM records.", auth: Auth::ApiToken, category: "CRM", brand_color: "#2d7ff9", two_way: false, implemented: true,
        fields: KEY_F, instructions: &["Attio → Workspace settings → API keys"], aliases: &["crm", "sales"] },
    Connector { id: "close", title: "Close", blurb: "Sales CRM leads and calls.", auth: Auth::ApiToken, category: "CRM", brand_color: "#276392", two_way: false, implemented: true,
        fields: KEY_F, instructions: &["Close → Settings → API Keys"], aliases: &["crm", "sales"] },
    Connector { id: "apollo", title: "Apollo.io", blurb: "Prospect and enrich contacts.", auth: Auth::ApiToken, category: "Sales", brand_color: "#fbbf24", two_way: false, implemented: true,
        fields: KEY_F, instructions: &["Apollo → Settings → Integrations → API"], aliases: &["prospecting", "leads", "sales"] },
    Connector { id: "hunter", title: "Hunter", blurb: "Find and verify email addresses.", auth: Auth::ApiToken, category: "Sales", brand_color: "#fa5320", two_way: false, implemented: true,
        fields: KEY_F, instructions: &["Hunter → API → copy your key"], aliases: &["email", "leads", "prospecting"] },
    Connector { id: "clay", title: "Clay", blurb: "Enrichment workflows.", auth: Auth::ApiToken, category: "Sales", brand_color: "#1f2328", two_way: false, implemented: false,
        fields: KEY_F, instructions: &["Clay → Settings → API"], aliases: &["enrichment", "leads"] },

    // ---- support ----
    Connector { id: "zendesk", title: "Zendesk", blurb: "Support tickets and customers.", auth: Auth::ApiToken, category: "Support", brand_color: "#03363d", two_way: false, implemented: true,
        fields: &[f("email", "Account email", false, ""), f("api_token", "API token", true, ""), f("subdomain", "Subdomain", false, "yourcompany")],
        instructions: &["Admin Center → Apps and integrations → Zendesk API", "Enable token access and add an API token"], aliases: &["tickets", "support", "helpdesk"] },

    // ---- finance ----
    Connector { id: "stripe", title: "Stripe", blurb: "Payments, customers and payouts.", auth: Auth::ApiToken, category: "Finance", brand_color: "#635bff", two_way: false, implemented: true,
        fields: &[f("api_key", "Secret key", true, "sk_live_… / sk_test_…")],
        instructions: &["Open dashboard.stripe.com/apikeys", "Use a restricted read-only key where possible"], aliases: &["payments", "billing", "revenue"] },
    Connector { id: "quickbooks", title: "QuickBooks", blurb: "Accounting, invoices and expenses.", auth: Auth::OAuth, category: "Finance", brand_color: "#2ca01c", two_way: false, implemented: true,
        fields: &[], instructions: &["Create an app in the Intuit developer portal", "Authorize your company file"], aliases: &["accounting", "invoices", "expenses"] },

    // ---- files / design ----
    Connector { id: "dropbox", title: "Dropbox", blurb: "Read and write your files.", auth: Auth::OAuth, category: "Files", brand_color: "#0061ff", two_way: false, implemented: true,
        fields: &[], instructions: &["Create an app at dropbox.com/developers/apps", "Generate an access token or authorize the app"], aliases: &["files", "storage"] },
    Connector { id: "box", title: "Box", blurb: "Enterprise file storage.", auth: Auth::OAuth, category: "Files", brand_color: "#0061d5", two_way: false, implemented: true,
        fields: &[], instructions: &["Create an app in the Box developer console", "Authorize it for your account"], aliases: &["files", "storage"] },
    Connector { id: "figma", title: "Figma", blurb: "Read design files and comments.", auth: Auth::ApiToken, category: "Design", brand_color: "#f24e1e", two_way: false, implemented: true,
        fields: TOKEN_F, instructions: &["Figma → Settings → Personal access tokens"], aliases: &["design", "mockups"] },
    Connector { id: "canva", title: "Canva", blurb: "Designs and brand assets.", auth: Auth::OAuth, category: "Design", brand_color: "#00c4cc", two_way: false, implemented: true,
        fields: &[], instructions: &["Create an app in the Canva developer portal", "Authorize it"], aliases: &["design", "graphics"] },
    Connector { id: "descript", title: "Descript", blurb: "Transcripts and video projects.", auth: Auth::ApiToken, category: "Design", brand_color: "#0062ff", two_way: false, implemented: false,
        fields: KEY_F, instructions: &["Descript → Settings → API"], aliases: &["video", "transcript", "audio"] },
    Connector { id: "docusign", title: "Docusign", blurb: "Send and track signature requests.", auth: Auth::OAuth, category: "Docs", brand_color: "#4c00ff", two_way: false, implemented: true,
        fields: &[], instructions: &["Create an integration key in the Docusign admin", "Authorize your account"], aliases: &["signature", "contracts", "esign"] },

    // ---- analytics ----
    Connector { id: "posthog", title: "PostHog", blurb: "Product analytics and funnels.", auth: Auth::ApiToken, category: "Analytics", brand_color: "#f54e00", two_way: false, implemented: true,
        fields: &[f("api_key", "Personal API key", true, "phx_…"), f("host", "Host", false, "https://app.posthog.com")],
        instructions: &["PostHog → Settings → Personal API keys"], aliases: &["analytics", "events", "product"] },
    Connector { id: "mixpanel", title: "Mixpanel", blurb: "Event analytics.", auth: Auth::ApiToken, category: "Analytics", brand_color: "#7856ff", two_way: false, implemented: true,
        fields: &[f("username", "Service account username", false, ""), f("secret", "Service account secret", true, "")],
        instructions: &["Mixpanel → Organization Settings → Service Accounts"], aliases: &["analytics", "events"] },
    Connector { id: "amplitude", title: "Amplitude", blurb: "Product analytics charts and cohorts.", auth: Auth::ApiToken, category: "Analytics", brand_color: "#1e61f0", two_way: false, implemented: true,
        fields: &[f("api_key", "API key", true, ""), f("secret_key", "Secret key", true, "")],
        instructions: &["Amplitude → Settings → Projects → your project", "Copy the API key and secret key"], aliases: &["analytics", "events"] },

    // ---- automation surfaces ----
    Connector { id: "browser", title: "Browser", blurb: "Drive a real browser for sites without an API.", auth: Auth::None, category: "Automation", brand_color: "#0ea5e9", two_way: false, implemented: false,
        fields: &[], instructions: &["No credentials needed — the assistant drives a local browser session"], aliases: &["web", "scrape", "automation"] },
];

pub fn find(id: &str) -> Option<&'static Connector> {
    CONNECTORS.iter().find(|c| c.id == id)
}
