import { Msg, OwRail, OwWin, SlackComposer, SlackDate, SlackRail, SlackWin, Sticky, WS_NAME, d } from "./OwRail";

/* ---- scene 3: a teammate's first mention waits for your OK ---- */
export function SceneTeammates() {
  return (
    <>
      <span className="hiw-spark" style={d("1.9s")} />
      <Sticky d="3.4s" pos={{ left: "53%", bottom: "10%" }}>first-time senders wait for your OK</Sticky>
      <SlackWin>
        <SlackRail active="launch-room" />
        <div className="hiw-slmain">
          <div className="hiw-slhead"># launch-room <span className="hiw-sub">· 24 members</span></div>
          <div className="hiw-slmsgs">
            <SlackDate label="Today" />
            <Msg
              av="P" avBg="#7c6cd0" name="Priya N" ts="6:41 PM" delay=".7s"
              extra={
                <span className="hiw-replybar hiw-k" style={d("5.6s")}>
                  <span className="hiw-sav2">OW</span> 1 reply
                  <span className="hiw-later">after you allow</span>
                </span>
              }
            >
              <span className="hiw-men">@Superuser</span> pull the signup numbers?
            </Msg>
          </div>
          <SlackComposer placeholder="Message #launch-room" />
        </div>
      </SlackWin>
      <OwWin>
        <OwRail hot="Summarize #launch-room" hotSub="via Slack" />
        <div className="hiw-owmain">
          <div className="hiw-owtitle">Slack — {WS_NAME}</div>
          <div className="hiw-waitrow hiw-k hiw-glow" style={d("2s", { "--g": "2.5s" })}>
            <span className="min-w-0"><b>Priya N</b> is waiting</span>
            <span className="hiw-allowbtn ml-auto">Allow &amp; deliver</span>
          </div>
          <div className="hiw-waitcap hiw-k" style={d("3.4s")}>
            Each teammate&apos;s <b>first</b> mention waits for your OK — then they&apos;re on the People list and it flows.
          </div>
        </div>
      </OwWin>
    </>
  );
}
