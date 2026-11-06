import type { ReactNode } from "react";
import type { ProviderInfo } from "../../api";
import { timeAgo } from "../../when";
import { PROVIDER_LOGOS } from "../logos";

// The provider gallery and its key form, shared by step 1 of onboarding (§39) and Settings ▸
// Models (UX-021) so the two can't drift apart. The hook runs the interaction; ProviderCards
// and ProviderForm are the shared markup. Each surface keeps its own frame (a fixed-height
// dialog, a scrolling page) and passes a test-id prefix so e2e can address each separately.

/** Where someone who isn't a developer gets an API key: the page, and how to name it. */
export const KEY_HELP: Record<string, { url: string; label: string }> = {
  anthropic: { url: "https://console.anthropic.com/settings/keys", label: "console.anthropic.com" },
  bedrock: { url: "https://console.aws.amazon.com/bedrock/home#/api-keys", label: "the AWS Bedrock console" },
  deepseek: { url: "https://platform.deepseek.com/api_keys", label: "platform.deepseek.com" },
  fireworks: { url: "https://fireworks.ai/account/api-keys", label: "fireworks.ai" },
  gemini: { url: "https://aistudio.google.com/apikey", label: "aistudio.google.com" },
  kimi: { url: "https://platform.moonshot.ai/console/api-keys", label: "platform.moonshot.ai" },
  minimax: { url: "https://platform.minimax.io", label: "platform.minimax.io" },
  mistral: { url: "https://console.mistral.ai/api-keys", label: "console.mistral.ai" },
  openai: { url: "https://platform.openai.com/api-keys", label: "platform.openai.com" },
  openrouter: { url: "https://openrouter.ai/keys", label: "openrouter.ai" },
  qwen: { url: "https://modelstudio.console.alibabacloud.com", label: "alibabacloud.com" },
  together: { url: "https://api.together.xyz/settings/api-keys", label: "together.xyz" },
  xai: { url: "https://console.x.ai", label: "console.x.ai" },
  zai: { url: "https://z.ai/manage-apikey/apikey-list", label: "z.ai" },
};

export type Verify = { state: "idle" | "testing" | "ok" | "error"; msg?: string };

/** A provider's tile: its logo on a light plate, or its first letter.
 *
 *  Only a logo gets the plate. The marks are dark artwork and need a light ground in either
 *  theme, but a column of white tiles beside providers without a logo was the brightest thing
 *  on a dark screen, pulling the eye to the part of the page with nothing to decide. The letter
 *  sits on the app's own surface. */
export function ProviderMark({ name, title, size = 32 }: { name: string; title: string; size?: number }) {
  const logo = PROVIDER_LOGOS[name];
  const mark = size * 0.6;
  return (
    <span className={`border border-line grid place-items-center rounded-lg shrink-0 ${logo ? "provider-plate" : "bg-paper"}`} style={{ width: size, height: size }}>
      {logo ? (
        <img src={logo} alt="" style={{ width: mark, height: mark }} />
      ) : (
        <span className="font-semibold text-md text-muted">{title.charAt(0).toUpperCase()}</span>
      )}
    </span>
  );
}

/** When a provider last answered, in the app's one way of saying it; null if never. */
export function relTime(epoch?: number | null): string | null {
  return epoch ? timeAgo(epoch) : null;
}

/** What `useProviderSetup` gives the gallery and the form. */
export interface ProviderSetupState {
  providers: ProviderInfo[];
  ordered: ProviderInfo[];
  refreshProviders: () => Promise<void>;
  sel: string | null;
  info: ProviderInfo | undefined;
  openProvider: (name: string) => void;
  backToGallery: () => void;
  cancelBackTimer: () => void;
  fields: Record<string, string>;
  setFieldValue: (key: string, value: string) => void;
  dirty: boolean;
  credentialed: boolean;
  savedState: boolean;
  secretFilled: boolean;
  keylessOk: Set<string>;
  showEndpoint: boolean;
  setShowEndpoint: (v: boolean) => void;
  verify: Verify;
  runTestAndSave: () => Promise<boolean>;
  removeKey: () => Promise<void>;
  statusFor: (p: ProviderInfo, opts?: { lastUsed?: boolean }) => ReactNode;
  /** Saves a non-secret field of a provider that is already set up when it loses focus. Test is
   *  how a KEY is saved; extras such as Anthropic's thinking_budget mustn't need a new test
   *  (owner, 2026-07-23: the budget was never saved and nothing said so). */
  saveField: (key: string) => Promise<void>;
  /** The field showing "✓ Saved" just now. */
  fieldSaved: string | null;
}
