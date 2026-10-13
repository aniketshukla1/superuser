import { timeAgo } from "../../when";

// The providers' Last-used line (null when never used), in the app's one vocabulary for time.
export const relTime = (epoch?: number | null): string | null => (epoch ? timeAgo(epoch) : null);

// Shared tab bodies for the Settings and Integrations pages (the old top-tab ManageModal was retired
// when Settings/Activity became full-page surfaces): ModelsTab → Settings ▸ Models; ConnectorsTab +
// McpTab → Integrations ▸ Connectors / MCP servers.
export const BTN_BORDERED =
  "ctl border border-line bg-paper hover:border-lineStrong shrink-0";
export const BTN_ACCENT = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-50";
export const BTN_DANGER = "text-sm text-danger/80 hover:text-danger shrink-0";

/** Two-letter initials for a chip/avatar (first+last word, else first two chars). */
export function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "?";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

export const EXAMPLE = `{
  "filesystem": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/dir"],
    "enabled": true
  }
}`;
