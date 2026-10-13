import type { Connector, SlackStatus } from "../../../api";
import { CHIP_OK, CHIP_OFF, CHIP_WARN } from "../ui";

export function healthChip(c: Connector, slack: SlackStatus | null) {
  // Slack relay gets a LIVE chip from /v1/connectors/slack/status — problems
  // surface in the list, never one click deep. Named honestly per layer; we
  // never claim "Slack↔cloud down" (the desktop can't see that leg).
  if (c.name === "slack" && c.mode === "relay" && slack) {
    if (!slack.signed_in) return <span className={CHIP_WARN}>● Sign-in needed</span>;
    if (slack.relay.state === "offline") return <span className={CHIP_OFF}>● Offline</span>;
    if (slack.relay.state === "reconnecting")
      return <span className={CHIP_WARN}>● Reconnecting</span>;
    if (Object.values(slack.teams).some((t) => !t.token_ok))
      return <span className={CHIP_WARN}>● Token not working</span>;
    return <span className={CHIP_OK}>● Live</span>;
  }
  if (c.two_way && c.connected) return <span className={CHIP_OK}>● Live</span>;
  return <span className={CHIP_OK}>● Ready</span>;
}
