import { Msg, OwWin, SlackComposer, SlackDate, SlackRail, SlackWin, Sticky, d } from "./OwRail";

/* ---- scene 2: mention INSIDE the open thread panel → the same session ---- */
export function SceneThread({ meFirst, meInitial }: { meFirst: string; meInitial: string }) {
  return (
    <>
      <span className="hiw-spark" style={d("1.9s")} />
      <Sticky d="3.2s" r pos={{ left: "52%", top: "10%" }}>chatting in the thread continues the SAME conversation →</Sticky>
      <SlackWin>
        <SlackRail active="launch-room" />
        <div className="hiw-slmain">
          <div className="hiw-slhead"># launch-room <span className="hiw-sub">· 24 members</span></div>
          <div className="hiw-slmsgs">
            <SlackDate label="Today" />
            <Msg av="P" avBg="#7c6cd0" name="Priya N" ts="6:31 PM">
              signups are spiking since the post 📈
            </Msg>
            <Msg
              av={meInitial} avBg="#3b82c4" name={meFirst} ts="6:33 PM"
              extra={
                <span className="hiw-replybar">
                  <span className="hiw-sav2">OW</span> 2 replies
                  <span className="hiw-later">Today at 6:36 PM</span>
                </span>
              }
            >
              <span className="hiw-men">@Superuser</span> summarize this thread
            </Msg>
          </div>
          <SlackComposer placeholder="Message #launch-room" />
          {/* thread panel open from the start — the new mentions play INSIDE it */}
          <div className="hiw-slthread">
            <div className="hiw-th">Thread <span className="hiw-sub"># launch-room</span><span className="hiw-x">✕</span></div>
            <div className="hiw-tmsgs">
              <Msg av={meInitial} avBg="#3b82c4" name={meFirst} ts="6:33 PM">
                <span className="hiw-men">@Superuser</span> summarize this thread
              </Msg>
              <div className="hiw-cnt">2 replies</div>
              <Msg av="OW" avBg="#4a154b" name="Superuser" app ts="6:34 PM">
                Launch traction: signups up 3.4×…
              </Msg>
              <Msg av="P" avBg="#7c6cd0" name="Priya N" ts="6:36 PM" delay=".8s">
                <span className="hiw-men">@Superuser</span> break it down by country?
              </Msg>
              <Msg av="OW" avBg="#4a154b" name="Superuser" app ts="6:36 PM" delay="4.8s">
                Top: US 41% · India 22% · Germany 9%…
              </Msg>
            </div>
            <div className="hiw-treply">Reply…</div>
          </div>
        </div>
      </SlackWin>
      <OwWin>
        <div className="hiw-owrail">
          <div className="hiw-brand">Superuser</div>
          <div className="hiw-newbtn">＋ New session</div>
          <div className="hiw-ownav">⌕ Search</div>
          <div className="hiw-ownav">◷ Automations</div>
          <div className="hiw-sect">RECENT</div>
          <div className="hiw-sess hot hiw-stay hiw-glow" style={{ "--g": "2.4s" } as React.CSSProperties}>
            <b>Summarize #launch-room</b>via Slack
          </div>
          <div className="hiw-sess"><b>Jira vs Linear</b>Assistant</div>
        </div>
        <div className="hiw-owmain">
          <div className="hiw-owtitle">
            Summarize #launch-room <span className="hiw-via">via Slack — same session</span>
          </div>
          <div className="hiw-owchat">
            <div className="hiw-bub agent hiw-stay">…signups up 3.4×, top referrer is the press page.</div>
            <div className="hiw-bub user hiw-k" style={d("2.6s")}>break it down by country?</div>
            <div className="hiw-bub agent hiw-k" style={d("3.8s")}>
              Top countries: US 41%, India 22%, Germany 9% — context kept from the whole thread.
            </div>
          </div>
          <div className="hiw-owcomposer">Message Superuser…</div>
        </div>
      </OwWin>
    </>
  );
}
