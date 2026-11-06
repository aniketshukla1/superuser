import { useEffect, useRef, useState } from "react";
import { getProviders, removeProvider, setProvider, verifyProvider, type ProviderInfo } from "../../api";
import { providerRank } from "../logos";
import { relTime, type ProviderSetupState, type Verify } from "./ProviderSetupState";

type Fields = Record<string, string>;

const IDLE: Verify = { state: "idle" };
const RETURN_AFTER_SAVE_MS = 900;
const SAVED_FLASH_MS = 1400;

/** Configured providers first, then the house order. Nine providers never set up used to sit
 *  above the two in use; Connectors, the next surface along, lists connected ones first, and
 *  this is the same kind of list. */
function orderProviders(list: ProviderInfo[]): ProviderInfo[] {
  return [...list].sort((a, b) => Number(Boolean(b.configured)) - Number(Boolean(a.configured)) || providerRank(a.name) - providerRank(b.name));
}

/** The status line on a provider's card. */
function providerStatus(p: ProviderInfo, running: boolean, lastUsed: boolean) {
  if (p.configured && p.needs_key) {
    const used = lastUsed ? relTime(p.last_used_at) : null;
    return (
      <span className="block font-medium text-ok text-xs truncate">
        ✓ Connected{used ? <span className="font-normal text-muted"> · used {used}</span> : ""}
      </span>
    );
  }
  if (p.needs_key) return <span className="block text-faint text-xs truncate">Not set up</span>;
  return <span className="block text-faint text-xs truncate">{running ? <span className="font-medium text-ok">✓ Running</span> : "No key needed"}</span>;
}

/** The gallery ⇄ key form state machine shared by onboarding and Settings ▸ Models. */
export function useProviderSetup(opts?: { onSaved?: () => void }): ProviderSetupState {
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  /** null shows the gallery; a provider's name shows its key form. */
  const [sel, setSel] = useState<string | null>(null);
  const [fields, setFields] = useState<Fields>({});
  const [dirty, setDirty] = useState(false);
  const [showEndpoint, setShowEndpoint] = useState(false);
  const [verify, setVerify] = useState<Verify>(IDLE);
  /** Keyless providers (Ollama) say they're configured without proving anything runs; passing
   *  Detect in this session is what marks them running. */
  const [keylessOk, setKeylessOk] = useState<Set<string>>(new Set());
  /** Unsaved input per provider, kept while moving between cards (owner, 2026-07-16). */
  const [drafts, setDrafts] = useState<Record<string, Fields>>({});
  /** The non-secret field that just saved on blur, which shows "✓ Saved". */
  const [fieldSaved, setFieldSaved] = useState<string | null>(null);
  const backTimer = useRef<number | null>(null);
  const savedTimer = useRef<number | null>(null);

  const refreshProviders = () => getProviders().then(setProviders).catch(() => {});
  const cancelBackTimer = () => {
    if (backTimer.current) window.clearTimeout(backTimer.current);
  };
  const keepDraft = (name: string, draft: Fields) => setDrafts((all) => ({ ...all, [name]: draft }));
  const afterSave = async () => {
    await refreshProviders();
    opts?.onSaved?.();
  };

  useEffect(() => {
    refreshProviders();
    return cancelBackTimer;
  }, []);

  const info = providers.find((p) => p.name === sel);
  const credentialed = Boolean(info?.configured) && Boolean(info?.needs_key);

  const openProvider = (name: string) => {
    if (sel) keepDraft(sel, fields);
    const provider = providers.find((p) => p.name === name);
    const draft = drafts[name];
    const start: Fields = {};
    for (const field of provider?.fields ?? []) start[field.key] = draft?.[field.key] || provider?.values?.[field.key] || field.default || "";
    setSel(name);
    setFields(start);
    setDirty(Boolean(draft) && Object.values(draft).some(Boolean));
    setVerify(IDLE);
    setShowEndpoint(false);
  };

  const backToGallery = () => {
    // Keep only UNSAVED input. Keeping it every time once caught the key just saved on the way
    // back after Test, so reopening a connected provider put the key back into the field in
    // plain text instead of the masked placeholder and saved pill (owner, 2026-07-19). A clean
    // form clears any old draft.
    if (sel) keepDraft(sel, dirty ? fields : {});
    setSel(null);
    setVerify(IDLE);
  };

  // Test checks, saves and goes back (§39): a passing Test saves and returns to the gallery,
  // where the card now shows its ✓, with no more clicks.
  const runTestAndSave = async (): Promise<boolean> => {
    if (!sel) return false;
    setVerify({ state: "testing" });
    const checked = await verifyProvider(sel, fields).catch(() => ({ ok: false, error: "unreachable" }));
    if (!checked.ok) {
      setVerify({ state: "error", msg: checked.error || "couldn't verify" });
      return false;
    }
    if (dirty || !info?.configured) await setProvider(sel, fields).catch(() => {});
    if (!info?.needs_key) setKeylessOk((ok) => new Set(ok).add(sel));
    setVerify({ state: "ok" });
    setDirty(false);
    keepDraft(sel, {});
    await afterSave();
    // A moment for "✓ Tested & saved" to register, then back. Not via backToGallery: the timer
    // would run its old closure (dirty and fields from before the save) and keep the saved key
    // as a draft, the bug above. This way back always clears the draft.
    backTimer.current = window.setTimeout(() => {
      keepDraft(sel, {});
      setSel(null);
      setVerify(IDLE);
    }, RETURN_AFTER_SAVE_MS);
    return true;
  };

  // On a provider already set up, a non-secret field saves when it loses focus: extras like
  // Anthropic's thinking_budget must stick without testing the key again (owner, 2026-07-23:
  // typed, left Settings, never saved). Secrets still save only through Test, and a provider
  // not yet set up saves everything with its first Test.
  const saveField = async (key: string) => {
    if (!sel || !info?.configured) return;
    const spec = info.fields.find((field) => field.key === key);
    if (!spec || spec.secret) return;
    const typed = (fields[key] || "").trim();
    if (typed === (info.values?.[key] || "").trim()) return;
    const saved = await setProvider(sel, { [key]: typed }).catch(() => ({ ok: false }));
    if (!saved.ok) return;
    await afterSave();
    setFieldSaved(key);
    if (savedTimer.current) window.clearTimeout(savedTimer.current);
    savedTimer.current = window.setTimeout(() => setFieldSaved(null), SAVED_FLASH_MS);
  };

  // Settings only: forget the stored key, and the card goes back to "Not set up".
  const removeKey = async () => {
    if (!sel) return;
    await removeProvider(sel).catch(() => {});
    keepDraft(sel, {});
    setKeylessOk((ok) => {
      const left = new Set(ok);
      left.delete(sel);
      return left;
    });
    await afterSave();
    setSel(null);
    setVerify(IDLE);
  };

  const setFieldValue = (key: string, value: string) => {
    setFields((current) => ({ ...current, [key]: value }));
    setDirty(true);
    setVerify(IDLE);
  };

  return {
    providers,
    ordered: orderProviders(providers),
    refreshProviders,
    sel,
    info,
    openProvider,
    backToGallery,
    cancelBackTimer,
    fields,
    setFieldValue,
    dirty,
    credentialed,
    // The saved look inside the key box (§39), a green border and pill, for a stored key or a
    // fresh pass alike; typing clears it.
    savedState: (credentialed && !dirty) || verify.state === "ok",
    // Only REQUIRED secrets hold back Test: cloud providers (Bedrock, Vertex) have optional key
    // fields, with the credentials perhaps in ~/.aws or ADC instead.
    secretFilled: (info?.fields ?? []).every((field) => !field.secret || !field.required || (fields[field.key] || "").trim() !== ""),
    keylessOk,
    showEndpoint,
    setShowEndpoint,
    verify,
    runTestAndSave,
    removeKey,
    statusFor: (p, o) => providerStatus(p, keylessOk.has(p.name), Boolean(o?.lastUsed)),
    saveField,
    fieldSaved,
  };
}
