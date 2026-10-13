// Where your data goes — computed from what is actually configured, said the same way
// everywhere it is said.
//
// Four screens made four claims and they did not agree:
//   • Settings ▸ Privacy: "there is no account, no sync, and no telemetry" and "No cloud key
//     is set, so every prompt is answered by a model running on this machine";
//   • the account menu: "Sign in to Superuser Cloud";
//   • Settings ▸ Models: "OpenCode Zen ✓ Connected";
//   • Privacy: 3.3 MB sent to opencode.ai "for model".
// Each was true of the thing it checked. The Settings card read one flag (`has_key`) that
// counts some providers and not others, and it stated the signed-out case as if it were the
// only case — signed-in accounts do report product telemetry unless it is switched off.
//
// For an app whose whole claim is that it keeps your data at home, a contradiction on this
// subject is the worst kind. So this derives the statement from the providers, the account
// and local-only mode, and each place that talks about it renders the same result.

import type { CloudStatus, ProviderInfo } from "./api";

/** Providers that answer on this machine without a key. Anything else with a key — or a
 *  subscription login like Codex — sends prompts to someone else's servers. */
const LOCAL_PROVIDERS = new Set(["ollama", "lmstudio", "llamacpp", "local"]);

/** A custom endpoint on loopback is a local server wearing an OpenAI-compatible API. */
const isLoopback = (url?: string) => !!url && /^(https?:\/\/)?(localhost|127\.0\.0\.1|\[?::1\]?)(:\d+)?(\/|$)/i.test(url.trim());

/** The configured providers that send prompts off this machine, by title. */
export function cloudProviders(providers: ProviderInfo[]): string[] {
  return providers
    .filter((p) => p.configured)
    .filter((p) => !LOCAL_PROVIDERS.has(p.name))
    .filter((p) => !(p.name === "custom" && isLoopback(p.values?.base_url)))
    .map((p) => p.title);
}

/** "OpenAI", "OpenAI and Claude (Anthropic)", "A, B and C". Joined in words rather than set in
 *  parentheses: a provider title can carry its own ("Claude (Anthropic)"), and nested ones read
 *  "(OpenAI, Claude (Anthropic))". */
export function listInWords(items: string[]): string {
  if (items.length <= 1) return items.join("");
  return `${items.slice(0, -1).join(", ")} and ${items[items.length - 1]}`;
}

export type Tone = "local" | "leaves" | "neutral";

export interface DataFact {
  tone: Tone;
  text: string;
}

export function describeDataFlow({
  providers,
  cloud,
  localOnly,
}: {
  providers: ProviderInfo[] | null;
  cloud: CloudStatus | null;
  localOnly: boolean | null;
}): DataFact[] {
  const facts: DataFact[] = [
    { tone: "local", text: "Your conversations, memories and keys are stored on this Mac." },
  ];

  if (localOnly) {
    facts.push({ tone: "local", text: "Local-only mode is on, so nothing may leave this Mac — cloud models, connectors and web search are refused." });
  } else if (providers) {
    const out = cloudProviders(providers);
    facts.push(
      out.length
        ? {
            tone: "leaves",
            text: `${listInWords(out)} ${out.length === 1 ? "is" : "are"} connected. When a conversation uses a cloud model, what you send it goes to that provider; local models stay on this Mac.`,
          }
        : { tone: "local", text: "Only local models are set up, so your prompts are answered on this Mac." },
    );
  }

  if (cloud) {
    facts.push(
      cloud.signed_in
        ? {
            tone: cloud.telemetry_enabled === false ? "neutral" : "leaves",
            text: `Signed in to Superuser Cloud as ${cloud.account || "you"}, which brokers one-click connections. Product telemetry is ${cloud.telemetry_enabled === false ? "off" : "on"}.`,
          }
        : { tone: "local", text: "Not signed in to Superuser Cloud, so nothing is sent to Superuser — no account data, no telemetry." },
    );
  }
  return facts;
}
