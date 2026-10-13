use super::*;

#[derive(Debug, Clone, Copy)]
pub struct OAuthSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub scopes: &'static str,
    /// Extra query params on the authorize step (e.g. `access_type=offline`).
    pub extra_auth: &'static [(&'static str, &'static str)],
    /// API base for reads. `{instance}` is replaced by an instance url returned
    /// with the token (Salesforce), when present.
    pub api_base: &'static str,
    pub verify_path: &'static str,
    pub identity_at: &'static str,
    pub feeds: &'static [Feed],
    /// Vendors that reject a token request without a client secret.
    pub needs_secret: bool,
}

pub(super) const fn feed(path: &'static str, items_at: &'static str, id_key: &'static str, title_keys: &'static [&'static str], body_keys: &'static [(&'static str, &'static str)], kind: &'static str) -> Feed {
    Feed { path, items_at, id_key, title_keys, body_keys, kind, post_body: None }
}

/// A feed fetched with POST (Dropbox-style listing endpoints).
pub(super) const fn post_feed(path: &'static str, body: &'static str, items_at: &'static str, id_key: &'static str, title_keys: &'static [&'static str], body_keys: &'static [(&'static str, &'static str)], kind: &'static str) -> Feed {
    Feed { path, items_at, id_key, title_keys, body_keys, kind, post_body: Some(body) }
}

pub const SPECS: &[OAuthSpec] = &[
    OAuthSpec { id: "google_drive", name: "Google Drive", auth_url: "https://accounts.google.com/o/oauth2/v2/auth", token_url: "https://oauth2.googleapis.com/token",
        scopes: "https://www.googleapis.com/auth/drive.metadata.readonly", extra_auth: &[("access_type", "offline"), ("prompt", "consent")],
        api_base: "https://www.googleapis.com/drive/v3", verify_path: "/about?fields=user", identity_at: "user.emailAddress", needs_secret: true,
        feeds: &[feed("/files?pageSize=25&fields=files(id,name,mimeType,modifiedTime,webViewLink)", "files", "id", &["name"], &[("Type", "mimeType"), ("Modified", "modifiedTime"), ("Link", "webViewLink")], "file")] },
    OAuthSpec { id: "outlook", name: "Outlook", auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize", token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
        scopes: "offline_access User.Read Mail.Read Calendars.Read", extra_auth: &[],
        api_base: "https://graph.microsoft.com/v1.0", verify_path: "/me", identity_at: "userPrincipalName", needs_secret: false,
        feeds: &[
            feed("/me/messages?$top=25&$select=id,subject,from,receivedDateTime,bodyPreview", "value", "id", &["subject"], &[("From", "from"), ("Received", "receivedDateTime"), ("Preview", "bodyPreview")], "email"),
            feed("/me/events?$top=25&$select=id,subject,start,end,location", "value", "id", &["subject"], &[("Start", "start"), ("End", "end"), ("Location", "location")], "event"),
        ] },
    OAuthSpec { id: "box", name: "Box", auth_url: "https://account.box.com/api/oauth2/authorize", token_url: "https://api.box.com/oauth2/token",
        scopes: "root_readonly", extra_auth: &[], api_base: "https://api.box.com/2.0", verify_path: "/users/me", identity_at: "login", needs_secret: true,
        feeds: &[feed("/folders/0/items?limit=25", "entries", "id", &["name"], &[("Type", "type")], "file")] },
    OAuthSpec { id: "canva", name: "Canva", auth_url: "https://www.canva.com/api/oauth/authorize", token_url: "https://api.canva.com/rest/v1/oauth/token",
        scopes: "design:meta:read profile:read", extra_auth: &[], api_base: "https://api.canva.com/rest/v1", verify_path: "/users/me", identity_at: "team_user.user_id", needs_secret: true,
        feeds: &[feed("/designs?limit=25", "items", "id", &["title"], &[("Updated", "updated_at")], "design")] },
    OAuthSpec { id: "salesforce", name: "Salesforce", auth_url: "https://login.salesforce.com/services/oauth2/authorize", token_url: "https://login.salesforce.com/services/oauth2/token",
        scopes: "api refresh_token", extra_auth: &[], api_base: "{instance}/services/data/v60.0", verify_path: "/sobjects", identity_at: "", needs_secret: true,
        feeds: &[feed("/query?q=SELECT+Id,Name,Type,Industry+FROM+Account+ORDER+BY+LastModifiedDate+DESC+LIMIT+25", "records", "Id", &["Name"], &[("Type", "Type"), ("Industry", "Industry")], "account")] },
    OAuthSpec { id: "dropbox", name: "Dropbox", auth_url: "https://www.dropbox.com/oauth2/authorize", token_url: "https://api.dropboxapi.com/oauth2/token",
        scopes: "account_info.read files.metadata.read", extra_auth: &[("token_access_type", "offline")],
        // Dropbox exposes listing over POST only.
        api_base: "https://api.dropboxapi.com/2", verify_path: "", identity_at: "", needs_secret: true,
        feeds: &[post_feed("/files/list_folder", r#"{"path":"","limit":25}"#, "entries", "id", &["name"], &[("Type", ".tag"), ("Path", "path_display")], "file")] },
    OAuthSpec { id: "quickbooks", name: "QuickBooks", auth_url: "https://appcenter.intuit.com/connect/oauth2", token_url: "https://oauth.platform.intuit.com/oauth2/v1/tokens/bearer",
        // {instance} is the company id (realmId), captured from the OAuth callback.
        scopes: "com.intuit.quickbooks.accounting", extra_auth: &[], api_base: "https://quickbooks.api.intuit.com/v3/company/{instance}", verify_path: "", identity_at: "", needs_secret: true,
        feeds: &[feed("/query?query=select%20*%20from%20Invoice%20maxresults%2025&minorversion=70", "QueryResponse.Invoice", "Id", &["DocNumber"], &[("Total", "TotalAmt"), ("Customer", "CustomerRef"), ("Due", "DueDate")], "invoice")] },
    OAuthSpec { id: "docusign", name: "Docusign", auth_url: "https://account-d.docusign.com/oauth/auth", token_url: "https://account-d.docusign.com/oauth/token",
        // {instance} is the account's base uri + account id, resolved via userinfo.
        scopes: "signature", extra_auth: &[], api_base: "{instance}", verify_path: "", identity_at: "", needs_secret: true,
        feeds: &[feed("/envelopes?from_date=2020-01-01&count=25", "envelopes", "envelopeId", &["emailSubject"], &[("Status", "status"), ("Sent", "sentDateTime")], "envelope")] },
];

pub fn find_spec(id: &str) -> Option<&'static OAuthSpec> {
    SPECS.iter().find(|s| s.id == id)
}
