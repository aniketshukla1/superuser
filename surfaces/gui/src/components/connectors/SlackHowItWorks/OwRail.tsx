/* ---- shared miniature furniture ---- */

// The scenes deliberately play in a FICTIONAL workspace ("Lumina Labs") — a real
// account name here (or anything resembling our own product) reads as confusing
// or fake in an educational animation; the card's status line above keeps the
// user's real workspace name.
export const WS_NAME = "Lumina Labs";

/* Post-it notes (rev 7, owner-approved): the concept in five hand-written words,
   slapped onto the scene at its beat; they fade out near the end of each loop so
   they read as annotation, never as UI. */
export function Sticky({
  d: delay, r, pos, children,
}: {
  d: string; r?: boolean; pos: React.CSSProperties; children: React.ReactNode;
}) {
  return (
    <div
      className={"hiw-sticky hiw-k" + (r ? " r" : "")}
      style={{ "--d": delay, ...pos } as React.CSSProperties}
    >
      {children}
    </div>
  );
}

export const ThreadsIcon = () => (
  <svg className="hiw-ic" viewBox="0 0 24 24" aria-hidden="true">
    <path d="M21 11.5a8.5 8.5 0 1 1-4.7-7.6L21 3l-.9 4.7a8.5 8.5 0 0 1 .9 3.8z" />
    <path d="M8 10h8M8 14h5" />
  </svg>
);
export const SendIcon = () => (
  <svg className="hiw-ic" viewBox="0 0 24 24" aria-hidden="true">
    <path d="M22 2 11 13" />
    <path d="M22 2l-7 20-4-9-9-4 20-7z" />
  </svg>
);

export function SlackRail({ active }: { active: string }) {
  return (
    <div className="hiw-slrail">
      <div className="hiw-ws">{WS_NAME} ▾</div>
      <div className="hiw-slnav"><ThreadsIcon /> Threads</div>
      <div className="hiw-slnav"><SendIcon /> Drafts &amp; sent</div>
      <div className="hiw-sect">Channels</div>
      <div className={"hiw-ch" + (active === "general" ? " on" : "")}># general</div>
      <div className={"hiw-ch" + (active === "launch-room" ? " on" : "")}># launch-room</div>
      <div className="hiw-sect">Direct messages</div>
      <div className="hiw-slnav"><span className="hiw-pres" />Priya N</div>
      <div className="hiw-slnav"><span className="hiw-pres" />Emma W</div>
      <div className="hiw-sect">Agents &amp; apps</div>
      <div className="hiw-slnav"><span className="hiw-appav">OW</span>Superuser</div>
    </div>
  );
}

export function SlackWin({ children }: { children: React.ReactNode }) {
  return (
    <div className="hiw-win hiw-sl">
      <div className="hiw-sltop">
        <span className="hiw-dots"><i /><i /><i /></span>
        <span className="hiw-slsearch">⌕ Describe what you are looking for</span>
      </div>
      <div className="hiw-slbody">{children}</div>
    </div>
  );
}

/* Slack's two-row message box: formatting toolbar, then + / placeholder / send. */
export function SlackComposer({ placeholder }: { placeholder: string }) {
  return (
    <div className="hiw-slcomposer">
      <div className="hiw-slctools">
        <b>B</b><i>I</i><u>U</u><s>S</s><span>⛓</span><span>≔</span><span>≡</span><span>{"</>"}</span>
      </div>
      <div className="hiw-slcrow">
        <span className="hiw-plus">＋</span> {placeholder}
        <span className="hiw-send">➤</span>
      </div>
    </div>
  );
}

export function SlackDate({ label }: { label: string }) {
  return (
    <div className="hiw-sldate">
      <span>{label} ▾</span>
    </div>
  );
}

export function OwWin({ children }: { children: React.ReactNode }) {
  return (
    <div className="hiw-win hiw-ow">
      <div className="hiw-owtop">
        <span className="hiw-dots"><i /><i /><i /></span> Superuser
      </div>
      <div className="hiw-owbody">{children}</div>
    </div>
  );
}

export function OwRail({ hot, hotSub, glow }: { hot?: string; hotSub?: string; glow?: boolean }) {
  return (
    <div className="hiw-owrail">
      <div className="hiw-brand">Superuser</div>
      <div className="hiw-newbtn">＋ New session</div>
      <div className="hiw-ownav">⌕ Search</div>
      <div className="hiw-ownav">◷ Automations</div>
      <div className="hiw-sect">RECENT</div>
      {hot && (
        <div
          className={"hiw-sess hot" + (glow ? " hiw-glow hiw-k" : " hiw-stay")}
          style={glow ? ({ "--d": "2.5s", "--g": "2.9s" } as React.CSSProperties) : undefined}
        >
          <b>{hot}</b>
          {hotSub}
        </div>
      )}
      <div className="hiw-sess"><b>Jira vs Linear</b>Assistant</div>
    </div>
  );
}

export const d = (delay: string, extra?: Record<string, string>) =>
  ({ "--d": delay, ...extra } as React.CSSProperties);

export function Msg({
  av, avBg, name, ts, app, children, delay, extra,
}: {
  av: string; avBg: string; name: string; ts: string; app?: boolean;
  children: React.ReactNode; delay?: string; extra?: React.ReactNode;
}) {
  return (
    <div className={"hiw-slm" + (delay ? " hiw-k" : "")} style={delay ? d(delay) : undefined}>
      <span className="hiw-sav" style={{ background: avBg }}>{av}</span>
      <span className="min-w-0">
        <span className="hiw-nm">{name}{app && <span className="hiw-appb">APP</span>}</span>
        <span className="hiw-ts">{ts}</span>
        <br />
        <span>{children}</span>
        {extra}
      </span>
    </div>
  );
}
