import { useEffect, useRef, useState } from "react";
import type { SlackWorkspace } from "../../../api";
import { SceneMention } from "./SceneMention";
import { SceneThread } from "./SceneThread";
import { SceneTeammates } from "./SceneTeammates";

// UX-027: the post-connect "how mentions reach you" card. A tabbed carousel of
// animated split-scenes — Slack on the left (pinned to light-Slack colors, so it
// reads as a screenshot of Slack), Superuser on the right (app tokens). Tabs
// auto-advance through one full tour, then idle on a loop of the current scene;
// clicking a tab takes over. The chevron collapses the carousel to the status
// line — collapsed IS the seen-state (stored locally, survives restarts).
// "Listen to a channel" is deliberately absent: owner wants that scene reworked
// before it ships (UX-027 rev 4).

export const KEY = "ocw.slack.howitworks.collapsed";
export const DUR = 8000; // per-scene loop, ms
export const TABS = ["Mention → session", "Threads stay connected", "Allow teammates"];
export const CAPTIONS = [
  "Mention @Superuser in any channel it's invited to — a session opens here, and the answer lands back in Slack as a thread.",
  "Mention it again inside the thread — the conversation continues in the same session, context intact. The thread is the session.",
  "Teammates aren't auto-trusted: their first mention waits for your OK, then they're on the People list.",
];

export function readCollapsed(): boolean {
  try { return localStorage.getItem(KEY) === "1"; } catch { return false; }
}

export function SlackHowItWorks({ workspaces }: { workspaces: SlackWorkspace[] }) {
  const [collapsed, setCollapsed] = useState(readCollapsed);
  const [tab, setTab] = useState(0);
  const [cycle, setCycle] = useState(0); // bump = remount the scene = restart its animations
  const tourRef = useRef(TABS.length); // auto-advances left in the one-time story tour

  useEffect(() => {
    if (collapsed) return;
    const t = window.setTimeout(() => {
      if (tourRef.current > 1) {
        tourRef.current -= 1;
        setTab((x) => (x + 1) % TABS.length);
      } else {
        tourRef.current = 0;
        setCycle((c) => c + 1); // keep looping the current scene quietly
      }
    }, DUR);
    return () => window.clearTimeout(t);
  }, [tab, cycle, collapsed]);

  const jump = (i: number) => {
    tourRef.current = 0; // a click takes over: no more auto-advance
    setTab(i);
    setCycle((c) => c + 1);
  };
  const toggle = () => {
    const v = !collapsed;
    setCollapsed(v);
    try { localStorage.setItem(KEY, v ? "1" : "0"); } catch { /* best effort */ }
  };

  // Personalize when the install pre-added the connecting user (setup.py):
  // their workspace names the status line; the scenes call them by first name.
  const mine = workspaces.find(
    (w) => w.installer_user_id && w.allowed_users.includes(w.installer_user_id)
  );
  const ws = mine ?? workspaces[0];
  const meName =
    (mine &&
      (mine.installer_name ||
        mine.allowed_user_names?.[mine.installer_user_id ?? ""])) ||
    "You";
  const meFirst = meName.split(/\s+/)[0];
  const meInitial = (meName[0] || "Y").toUpperCase();

  return (
    // Rev 7 (owner): BORDERLESS — a proper section title + quiet status line;
    // the only boxes on screen are the two mini-windows, which own their frames.
    <div className="mb-5" data-testid="slack-howitworks">
      <div className="flex items-baseline gap-2.5">
        <h3 className="text-md font-semibold tracking-tight">
          Getting started with Slack &amp; Superuser
        </h3>
        <button
          className="ml-auto shrink-0 inline-flex items-center gap-1.5 text-sm text-muted hover:text-ink"
          data-testid="hiw-collapse"
          title={collapsed ? "Show how mentions work" : "Collapse — reopen anytime"}
          onClick={toggle}
        >
          {collapsed ? "How it works" : "Hide"}
          <span
            className="text-xs transition-transform"
            style={collapsed ? { transform: "rotate(-90deg)" } : undefined}
          >
            ▼
          </span>
        </button>
      </div>
      <div className="text-sm text-muted mt-0.5">
        <span className="text-ok font-bold">✓ </span>
        {ws?.account || "Workspace"} connected
        {mine
          ? " — you're on the People list, so your mentions get through."
          : " — here's how mentions reach you."}
      </div>

      {!collapsed && (
        <div className="mt-3">
          <div className="flex gap-1 border-b border-line mb-3">
            {TABS.map((t, i) => (
              <button
                key={t}
                className={"hiw-tab" + (i === tab ? " on" : "")}
                data-testid={`hiw-tab-${i}`}
                style={{ "--hiw-dur": `${DUR}ms` } as React.CSSProperties}
                onClick={() => jump(i)}
              >
                {t}
                <span className="hiw-prog"><i /></span>
              </button>
            ))}
          </div>

          <div className="hiw-scene hiw-play" key={`${tab}:${cycle}`} data-testid="hiw-scene">
            {tab === 0 && <SceneMention meFirst={meFirst} meInitial={meInitial} />}
            {tab === 1 && <SceneThread meFirst={meFirst} meInitial={meInitial} />}
            {tab === 2 && <SceneTeammates />}
          </div>
          <div className="mt-2.5 text-sm text-muted" data-testid="hiw-caption">
            {CAPTIONS[tab]}
          </div>
        </div>
      )}
    </div>
  );
}
