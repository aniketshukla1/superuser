import type { RecentSender } from "./superAgent";
import type { ConnectorTool } from "./waitForCloudSignIn";

// What the server says about each connector, and the accounts behind it.

export interface ConnectorField {
  key: string;
  label: string;
  help: string;
  placeholder: string;
  required: boolean;
  secret: boolean;
}

/** A message from someone not on the allow-list (yet), kept instead of dropped (§19). */
export interface ParkedMessage {
  id: string;
  ts: number;
  text: string;
  platform: string;
  chat_id: string;
  chat_name: string | null;
  chat_type: string;
  user_id: string;
  user_name: string | null;
  /** The workspace, for the managed Slack relay; null over manual Socket Mode. */
  team_id?: string | null;
}

/** A connected Slack workspace. The managed relay serves several, and ids only mean something
 *  inside their workspace, so each keeps its own allow-list. */
export interface SlackWorkspace {
  team_id: string;
  account: string;
  /** The slack.com subdomain, which stays unique when display names clash. */
  domain?: string;
  allow_all: boolean;
  allowed_users: string[];
  allowed_user_names?: Record<string, string | null>;
  approval_owner_ids?: string[];
  approval_owner_names?: Record<string, string | null>;
  /** Who installed it (authed_user). They join the allow-list on connect (UX-027); the GUI
   *  labels their chip "you" and words the setup card for them. */
  installer_user_id?: string;
  installer_name?: string;
}

/** A connected GitHub App installation. The managed relay serves several; logins are global,
 *  but each installation keeps its own allow-list. */
export interface GithubInstallation {
  installation_id: string;
  /** Where the App is installed: an organisation or a user. */
  account_login: string;
  /** "Organization" or "User". */
  account_type: string;
  /** "all" or "selected" repositories. */
  repo_selection: string;
  /** The login of the person who connected it. */
  github_login: string;
  allow_all: boolean;
  /** Logins allowed to start work. */
  allowed_users: string[];
}

/** A connected HubSpot portal (one `hubspot:portal:<hub_id>` profile each). */
export interface HubSpotPortal {
  hub_id: string;
  name: string;
  default: boolean;
  managed: boolean;
  sandbox: boolean;
  /** The consent level granted; "" for a token pasted by hand, where it isn't known. */
  access: "read" | "write" | "";
}

/** A connected Google account, the same shape for `gmail:account:<email>` and
 *  `google_calendar:account:<email>` profiles. */
export interface GmailAccount {
  email: string;
  scopes: string;
  default: boolean;
  managed: boolean;
  needs_reauth: boolean;
}

/** "Never show agents": enforced on this machine, in the tool layer. Agents just don't see the
 *  messages; the user sees how many were held back, on tool cards and in Activity. */
export interface GmailFilters {
  labels: string[];
  senders: string[];
}

/** One account of a connector that can hold several (`<name>:account:<id>` profiles: Notion
 *  workspaces, PostHog projects, …). Gmail and Calendar came first and keep the email-keyed
 *  shape above. */
export interface AccountRow {
  account_id: string;
  /** Who it is, as captured at connect: a workspace name, an email, … */
  name: string;
  default: boolean;
  managed: boolean;
}

export interface Connector {
  name: string;
  title: string;
  blurb: string;
  icon: string;
  /** A stable logo id for the client's registry; empty falls back to a glyph. */
  logo: string;
  /** A hex brand colour such as "#611f69"; "#6b7280" grey when there's none. */
  brand_color: string;
  /** More words that find it in the typeahead ("calendar" finds Outlook). */
  aliases?: string[];
  /** For the page shown before connecting (UX-DECISIONS §38): an optional About paragraph
   *  (left out when empty) and plain bullets on what it can reach. */
  about?: string;
  access?: string[];
  auth: string;
  fields: ConnectorField[];
  instructions: string[];
  available: boolean;
  connected: boolean;
  enabled: boolean;
  account: string | null;
  two_way: boolean;
  /** For chat platforms, narrower than two_way: sessions can subscribe to its channels. */
  channels: boolean;
  /** One-click through a vendor-hosted MCP server and local OAuth, with no cloud sign-in. */
  mcp?: boolean;
  /** One-click managed OAuth exists (it needs the cloud sign-in). */
  managed: boolean;
  /** One-click is off for now (Google's CASA review, say), shown as "Coming soon". */
  managed_paused?: boolean;
  /** The current profile came from managed OAuth rather than a pasted token. */
  managed_profile: boolean;
  /** "relay" on the managed cloud path; "" when connected by hand or with a token. */
  mode?: string;
  /** The allow-list, edited in place on the Connectors tab. */
  allowed_users: string[];
  /** Ids to display names, from the people directory. */
  allowed_user_names?: Record<string, string | null>;
  /** Manual Slack: the people who may answer approvals. */
  approval_owner_ids?: string[];
  approval_owner_names?: Record<string, string | null>;
  /** People seen lately on a connected two-way connector. */
  recent?: RecentSender[];
  /** Messages kept from senders not on the allow-list (§19). */
  unauthorized?: ParkedMessage[];
  tools: ConnectorTool[];
  /** Slack: the workspaces connected through the managed relay. */
  workspaces?: SlackWorkspace[];
  /** Gmail and Calendar use email-keyed rows; generic account connectors (notion, attio,
   *  posthog, …) use AccountRow. Each detail page narrows it for its connector. */
  accounts?: GmailAccount[] | AccountRow[];
  /** Gmail: the "Never show agents" senders and labels. */
  filters?: GmailFilters;
  /** HubSpot: the connected portals. */
  portals?: HubSpotPortal[];
  /** HubSpot: properties kept out of what agents read. */
  hidden_fields?: string[];
  /** GitHub: App installations through the managed relay. */
  installations?: GithubInstallation[];
}
