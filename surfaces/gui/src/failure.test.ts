import { describe, expect, it } from "vitest";
import { BRAIN_FAILURE_PREFIX, brainFailure, explainReason, noticeFailure } from "./failure";

// The reasons below are copied from real transcripts. Each used to reach the screen
// verbatim, inside an answer bubble.
describe("explainReason", () => {
  it("names a missing key, and points at the tab that exists now", () => {
    const r = explainReason("Add an API key for OpenAI in Settings → AI Models.");
    expect(r.summary).toBe("There's no API key for OpenAI yet.");
    expect(r.fix).toBe("models");
  });

  it("says which model sent back nothing", () => {
    expect(explainReason("Ollama (gemma4:12b) returned an empty response.").summary).toBe(
      "Ollama sent back an empty reply from gemma4:12b.",
    );
  });

  it("reads an out-of-credit refusal instead of printing its JSON", () => {
    const r = explainReason(
      'OpenCode Zen error 401 Unauthorized: {"type":"error","error":{"type":"CreditsError","message":"No payment method"}}',
    );
    expect(r.summary).toBe("OpenCode Zen refused the request: the account is out of credit.");
    expect(r.summary).not.toContain("{");
    expect(r.fix).toBe("models");
  });

  it("tells a local model that has not loaded apart from a server fault", () => {
    const r = explainReason(
      'Ollama error 500 Internal Server Error: {"error":"timed out waiting for llama-server to start - "}',
    );
    expect(r.summary).toMatch(/local model didn't start in time/);
    expect(r.fix).toBeUndefined();
  });

  it("handles a rejected key and rate limiting by status", () => {
    expect(explainReason("Anthropic error 401 Unauthorized").summary).toBe("Anthropic rejected the API key.");
    expect(explainReason("Groq error 429 Too Many Requests").summary).toMatch(/limiting requests/);
  });

  it("keeps a silent failure honest about being silent", () => {
    expect(explainReason("Codex: Codex failed without saying why.").summary).toBe("Codex stopped without saying why.");
  });

  it("falls back to the plain part of anything else, never the JSON", () => {
    expect(explainReason('model unreachable {"code": 7}').summary).toBe("Model unreachable.");
    expect(explainReason("").summary).toMatch(/no reason was given/);
  });
});

describe("brainFailure", () => {
  it("recognises the server's lead-in and keeps the reason as the detail", () => {
    const reason = "Ollama (gemma4:12b) returned an empty response.";
    const f = brainFailure(BRAIN_FAILURE_PREFIX + reason);
    expect(f?.title).toBe("Couldn't search your memory");
    expect(f?.detail).toBe(reason);
  });

  it("leaves a real answer alone", () => {
    expect(brainFailure("You've starred 18 GitHub repositories.")).toBeNull();
  });
});

describe("noticeFailure", () => {
  it("strips the Error: label the notice was stored with", () => {
    const f = noticeFailure("Error: model unreachable");
    expect(f.detail).toBe("model unreachable");
    expect(f.summary).toBe("Model unreachable.");
  });
});
