import { describe, expect, it } from "vitest";
import type { ProviderInfo } from "./api";
import { cloudProviders, describeDataFlow, listInWords } from "./dataFlow";

const provider = (over: Partial<ProviderInfo>): ProviderInfo => ({
  name: "x",
  title: "X",
  needs_key: true,
  fields: [],
  configured: false,
  values: {},
  suggested_models: [],
  recommended_model: null,
  ...over,
});

describe("cloudProviders", () => {
  it("counts every configured remote provider, including key-less subscriptions", () => {
    // The Settings card read `has_key`, which missed these — and said prompts stayed local.
    const got = cloudProviders([
      provider({ name: "ollama", title: "Ollama (local)", needs_key: false, configured: true }),
      provider({ name: "opencode", title: "OpenCode Zen", configured: true }),
      provider({ name: "codex", title: "Codex (ChatGPT plan)", needs_key: false, configured: true }),
      provider({ name: "anthropic", title: "Anthropic", configured: false }),
    ]);
    expect(got).toEqual(["OpenCode Zen", "Codex (ChatGPT plan)"]);
  });

  it("treats a custom endpoint on loopback as local", () => {
    expect(cloudProviders([provider({ name: "custom", title: "Custom", configured: true, values: { base_url: "http://127.0.0.1:11434/v1" } })])).toEqual([]);
    expect(cloudProviders([provider({ name: "custom", title: "Custom", configured: true, values: { base_url: "https://api.example.com/v1" } })])).toEqual(["Custom"]);
  });
});

describe("describeDataFlow", () => {
  it("says prompts leave when a cloud model is connected, and names it", () => {
    const facts = describeDataFlow({
      providers: [provider({ name: "opencode", title: "OpenCode Zen", configured: true })],
      cloud: { signed_in: false, account: "", user_id: "" },
      localOnly: false,
    });
    expect(facts.find((f) => f.tone === "leaves")?.text).toContain("OpenCode Zen");
    expect(facts[facts.length - 1]?.text).toMatch(/Not signed in/);
  });

  it("does not claim there is no telemetry for a signed-in account", () => {
    const facts = describeDataFlow({
      providers: [],
      cloud: { signed_in: true, account: "a@b.c", user_id: "u", telemetry_enabled: true },
      localOnly: false,
    });
    expect(facts[facts.length - 1]?.text).toContain("telemetry is on");
  });

  it("lets local-only mode override what is configured", () => {
    const facts = describeDataFlow({
      providers: [provider({ name: "opencode", title: "OpenCode Zen", configured: true })],
      cloud: null,
      localOnly: true,
    });
    expect(facts.some((f) => f.tone === "leaves")).toBe(false);
    expect(facts[1].text).toMatch(/Local-only mode is on/);
  });
});

describe("listInWords", () => {
  it("joins names in words, so a title's own parentheses aren't nested in another pair", () => {
    expect(listInWords(["OpenAI"])).toBe("OpenAI");
    expect(listInWords(["OpenAI", "Claude (Anthropic)"])).toBe("OpenAI and Claude (Anthropic)");
    expect(listInWords(["A", "B", "C"])).toBe("A, B and C");
    expect(listInWords([])).toBe("");
  });
});
