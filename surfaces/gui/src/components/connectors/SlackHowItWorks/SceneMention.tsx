import { Msg, OwRail, OwWin, SlackComposer, SlackDate, SlackRail, SlackWin, Sticky, d } from "./OwRail";

/* ---- scene 1: mention in a channel → new session, reply via thread panel ---- */
export function SceneMention({ meFirst, meInitial }: { meFirst: string; meInitial: string }) {
  return (
    <>
      <span className="hiw-spark" style={d("1.9s")} />
      <Sticky d="3.1s" pos={{ left: "51%", top: "8%" }}>a @mention starts a NEW session →</Sticky>
      <Sticky d="5.8s" r pos={{ left: "27%", bottom: "5%" }}>the answer comes back as a thread ↑</Sticky>
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
              av={meInitial} avBg="#3b82c4" name={meFirst} ts="6:33 PM" delay=".8s"
              extra={
                <span className="hiw-replybar hiw-k" style={d("4.6s")}>
                  <span className="hiw-sav2">OW</span> 1 reply
                  <span className="hiw-later">Today at 6:34 PM</span>
                </span>
              }
            >
              <span className="hiw-men">@Superuser</span> summarize this thread
            </Msg>
          </div>
          <SlackComposer placeholder="Message #launch-room" />
          <div className="hiw-slthread hiw-k" style={d("5.1s")}>
            <div className="hiw-th">Thread <span className="hiw-sub"># launch-room</span><span className="hiw-x">✕</span></div>
            <div className="hiw-tmsgs">
              <Msg av={meInitial} avBg="#3b82c4" name={meFirst} ts="6:33 PM">
                <span className="hiw-men">@Superuser</span> summarize this thread
              </Msg>
              <div className="hiw-cnt">1 reply</div>
              <Msg av="OW" avBg="#4a154b" name="Superuser" app ts="6:34 PM">
                Launch traction: signups up 3.4× since the post…
              </Msg>
            </div>
            <div className="hiw-treply">Reply…</div>
          </div>
        </div>
      </SlackWin>
      <OwWin>
        <OwRail hot="Summarize #launch-room" hotSub="via Slack · now" glow />
        <div className="hiw-owmain">
          <div className="hiw-owtitle hiw-k" style={d("2.6s")}>
            Summarize #launch-room <span className="hiw-via">via Slack</span>
          </div>
          <div className="hiw-owchat">
            <div className="hiw-bub user hiw-k" style={d("2.8s")}>@Superuser summarize this thread</div>
            <div className="hiw-bub agent hiw-k" style={d("3.6s")}>
              Reading the thread… signups up 3.4×, top referrer is the press page. <i>(replying in the Slack thread)</i>
            </div>
          </div>
          <div className="hiw-owcomposer">Message Superuser…</div>
        </div>
      </OwWin>
    </>
  );
}
