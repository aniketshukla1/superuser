import { CONNECTORS } from "./data";

/** Each connector's in-memory state for one test, and the payload it reports from it. */
export function connectorState() {
  const subscriptions: any[] = [
    // One existing subscription (a non-pinned session) so the Slack page's per-workspace
    // "Listening" row has an entry. Relay-mode channels are team-qualified (slack:T…/C…).
    { session_id: "wp-1", session_title: "Weekly plan 1", agent: "cowork", channel: "slack:T1DL/C0AAA111", channel_name: "ocw-test", routing_target: null, collision: false },
  ];
  // Parked unauthorized messages (§19) — mutable so Allow/Dismiss round-trip through the UI.
  // The relay is multi-workspace: parked items carry their team so the Slack page files them
  // under the right workspace card.
  const parked: any[] = [
    { id: "pk1", platform: "slack", chat_id: "C0AAA111", chat_name: "#ocw-test", user_id: "U0NEW", user_name: "Maya", chat_type: "channel", text: "hey ocw, can you summarize this thread?", ts: Date.now() / 1000 - 120, team_id: "T1DL" },
  ];
  // Slack connector — PER-TEST state (managed relay, two workspaces) so allow/disconnect
  // mutations never leak across tests sharing a worker. Backend parity: `workspaces` mirrors
  // the slack:team:* profiles, each with its OWN allow-list.
  const slackState = {
    connected: true,
    mode: "relay" as "" | "relay",
    account: "deeplearning.ai",
    allowed_users: [] as string[], // flat list (manual Socket Mode only)
    approval_owner_ids: [] as string[],
    workspaces: [
      // T1DL mirrors a managed install: the installer (authed_user) was pre-added
      // to the allow-list on connect (UX-027) — keys the "you" chip + setup card.
      { team_id: "T1DL", account: "deeplearning.ai", domain: "dlaiteam", allowed_users: ["U_ME"] as string[], allow_all: false, allowed_user_names: {} as Record<string, string | null>, approval_owner_ids: ["U_ME"] as string[], approval_owner_names: { U_ME: "Rohit Prasad" } as Record<string, string | null>, installer_user_id: "U_ME", installer_name: "Rohit Prasad" },
      { team_id: "T2AC", account: "acme-partners", domain: "acmehq", allowed_users: [] as string[], allow_all: false, allowed_user_names: {} as Record<string, string | null>, approval_owner_ids: [] as string[], approval_owner_names: {} as Record<string, string | null>, installer_user_id: "", installer_name: "" },
    ],
  };
  const slackConnector = () => ({
    name: "slack", title: "Slack", icon: "#", blurb: "Two-way Slack messaging.",
    auth: "bot_token", two_way: true, channels: true, available: true, brand_color: "#611f69", logo: "slack",
    fields: [], instructions: [], connected: slackState.connected,
    account: slackState.account, enabled: slackState.connected,
    allowed_users: [...slackState.allowed_users],
    approval_owner_ids: [...slackState.approval_owner_ids],
    tools: [], managed: true,
    managed_profile: slackState.mode === "relay", mode: slackState.mode,
    workspaces: slackState.workspaces.map((w) => ({ ...w, allowed_users: [...w.allowed_users] })),
    unauthorized: parked.map((x) => ({ ...x })),
  });
  // GitHub — PER-TEST multi-installation state (managed relay, one installation +
  // one parked mention) mirroring the backend's github:install:<id> profiles.
  const githubParked: any[] = [
    { id: "gh-pk1", platform: "github", chat_id: "acme/site#7", chat_name: "acme/site#7", user_id: "maya-dev", user_name: "maya-dev", chat_type: "channel", text: "@ocw please take a look at this flaky test", ts: Date.now() / 1000 - 90, team_id: "101" },
  ];
  const githubState = {
    connected: true,
    mode: "relay" as "" | "relay",
    installations: [
      { installation_id: "101", account_login: "acme", account_type: "Organization", repo_selection: "selected", github_login: "rohit-dev", allowed_users: ["rohit-dev"], allow_all: false },
    ],
  };
  const githubConnector = () => ({
    name: "github", title: "GitHub", icon: "⌘", blurb: "Work with issues, pull requests, repository files, and CI status.",
    auth: "token", two_way: true, channels: false, available: true, brand_color: "#1f2328", logo: "github",
    fields: [{ key: "token", label: "Personal access token", secret: true, required: true, help: "", placeholder: "" }],
    instructions: [], connected: githubState.connected,
    account: githubState.installations[0]?.account_login ?? null,
    enabled: githubState.connected, allowed_users: [], tools: [], managed: true,
    managed_profile: githubState.mode === "relay", mode: githubState.mode,
    installations: githubState.installations.map((i) => ({ ...i, allowed_users: [...i.allowed_users] })),
    unauthorized: githubParked.map((x) => ({ ...x })),
  });
  // Gmail — PER-TEST multi-account state (starts disconnected; managed connects add
  // mailboxes instantly, mirroring the backend's gmail:account:<email> profiles).
  // NOTE: the real server currently sends managed_paused: true for the Google trio
  // (CASA pending). The fixture keeps gmail UNPAUSED because the cloud-machinery specs
  // use its one-click as their subject; the paused UI is covered by google-paused.spec.ts
  // via a per-test connectors override.
  const gmailState = {
    accounts: [] as {
      email: string; default: boolean; managed: boolean; scopes: string; needs_reauth: boolean;
    }[],
    filters: { senders: [] as string[], labels: [] as string[] },
  };
  const GMAIL_NEXT = ["rohit@gmail.com", "work@dlai.com", "third@x.com"];
  const gmailConnector = () => {
    const base = CONNECTORS.connectors.find((c: any) => c.name === "gmail");
    return {
      ...base,
      connected: gmailState.accounts.length > 0,
      enabled: gmailState.accounts.length > 0,
      account: gmailState.accounts.find((a) => a.default)?.email ?? null,
      accounts: gmailState.accounts.map((a) => ({ ...a })),
      filters: { senders: [...gmailState.filters.senders], labels: [...gmailState.filters.labels] },
    };
  };
  // Google Calendar — PER-TEST multi-account state (gmail's shape, no filters).
  const gcalState = {
    accounts: [] as {
      email: string; default: boolean; managed: boolean; scopes: string; needs_reauth: boolean;
    }[],
  };
  const GCAL_NEXT = ["rohit@gmail.com", "work@dlai.com", "third@x.com"];
  const gcalConnector = () => {
    const base = CONNECTORS.connectors.find((c: any) => c.name === "google_calendar");
    return {
      ...base,
      connected: gcalState.accounts.length > 0,
      enabled: gcalState.accounts.length > 0,
      account: gcalState.accounts.find((a) => a.default)?.email ?? null,
      accounts: gcalState.accounts.map((a) => ({ ...a })),
    };
  };
  // Notion — PER-TEST generic multi-account state (accounts.py layer: AccountRow shape).
  const notionState = {
    accounts: [] as { account_id: string; name: string; default: boolean; managed: boolean }[],
  };
  const NOTION_NEXT = [
    { account_id: "ws-1", name: "Rohit's Workspace" },
    { account_id: "ws-2", name: "Ops Space" },
  ];
  const notionConnector = () => {
    const base = CONNECTORS.connectors.find((c: any) => c.name === "notion");
    return {
      ...base,
      connected: notionState.accounts.length > 0,
      enabled: notionState.accounts.length > 0,
      account: notionState.accounts.find((a) => a.default)?.name ?? null,
      accounts: notionState.accounts.map((a) => ({ ...a })),
    };
  };
  // Outlook — email-keyed managed accounts (mirrors outlook:account:<email> profiles).
  const outlookState = {
    accounts: [] as { account_id: string; name: string; default: boolean; managed: boolean }[],
  };
  // MCP-backed connectors (§42) — per-test connect state; the mock "browser flow"
  // completes instantly so the modal's poll picks it up.
  const mcpState = { monday: false, jira: false };
  const mcpConnector = (name: "monday" | "jira") => {
    const base = CONNECTORS.connectors.find((c: any) => c.name === name);
    return {
      ...base,
      connected: mcpState[name],
      enabled: mcpState[name],
      mode: mcpState[name] ? "mcp" : "",
    };
  };
  const outlookConnector = () => {
    const base = CONNECTORS.connectors.find((c: any) => c.name === "outlook");
    return {
      ...base,
      connected: outlookState.accounts.length > 0,
      enabled: outlookState.accounts.length > 0,
      account: outlookState.accounts.find((a) => a.default)?.name ?? null,
      accounts: outlookState.accounts.map((a) => ({ ...a })),
    };
  };
  // HubSpot — PER-TEST multi-portal state (starts disconnected; managed connects add
  // portals instantly, mirroring the backend's hubspot:portal:<hub_id> profiles).
  const hubspotState = {
    portals: [] as {
      hub_id: string; name: string; sandbox: boolean; default: boolean;
      managed: boolean; access: string;
    }[],
    hidden_fields: [] as string[],
    nextAccess: "read", // captured from the last connect-managed body
  };
  const HUBSPOT_NEXT = [
    { hub_id: "111", name: "Acme Inc", sandbox: false },
    { hub_id: "222", name: "Acme Sandbox", sandbox: true },
  ];
  const hubspotConnector = () => {
    const base = CONNECTORS.connectors.find((c: any) => c.name === "hubspot");
    return {
      ...base,
      connected: hubspotState.portals.length > 0,
      enabled: hubspotState.portals.length > 0,
      account: hubspotState.portals.find((p) => p.default)?.name ?? null,
      portals: hubspotState.portals.map((p) => ({ ...p })),
      hidden_fields: [...hubspotState.hidden_fields],
    };
  };

  return { subscriptions, parked, slackState, slackConnector, githubParked, githubState,
    githubConnector, gmailState, GMAIL_NEXT, gmailConnector, gcalState, GCAL_NEXT, gcalConnector,
    notionState, NOTION_NEXT, notionConnector, outlookState, mcpState, mcpConnector,
    outlookConnector, hubspotState, HUBSPOT_NEXT, hubspotConnector };
}
