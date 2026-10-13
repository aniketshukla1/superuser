import { describe, expect, it } from "vitest";
import { humanizeApprovalTitle, humanizeAsk, humanizeTool } from "./humanize";

// The email bot parks replies as `send_message` to `email:<address>`. Each place that names
// one reads as an email: "an email", sent to an address — not "a email message on email".
describe("an email target reads as an email", () => {
  const args = { target: "email:ada@example.com", text: "Re: Deck\n\nHere it is." };
  const line = (h: { pre: string; obj?: string; post?: string }) => `${h.pre}${h.obj ?? ""}${h.post ?? ""}`;

  it("in the approval title, the transcript and the ask", () => {
    expect(line(humanizeApprovalTitle("send_message", args))).toBe("Send an email to ada@example.com");
    expect(line(humanizeTool("send_message", args))).toBe("Sent an email to ada@example.com");
    expect(line(humanizeAsk("send_message", args))).toBe("Wanted to email ada@example.com");
  });

  it("leaves the chat channels as they were", () => {
    const slack = { target: "slack:T1/C9", text: "hi" };
    expect(line(humanizeApprovalTitle("send_message", slack))).toBe("Send a message to C9");
    expect(line(humanizeTool("send_message", slack))).toBe("Sent a Slack message to C9");
    expect(line(humanizeAsk("send_message", slack))).toBe("Wanted to message C9 on Slack");
  });
});
