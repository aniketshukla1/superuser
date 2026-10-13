import { useEffect, useState } from "react";
import { getSettings, removeModel, setDefaultModel, type ModelSettings, type ProviderInfo } from "../../api";
import { ModelChecklist } from "../ModelChecklist";
import { ProviderCards, ProviderForm, useProviderSetup } from "../../providers/ProviderSetup";
import { SEC_H } from "../../ui";
import { BackgroundModelCard, EmbeddingModelCard } from "./EmbeddingModelCard";

// -- Configure Models tab (UX-021: the shared provider gallery + key form) ----
// Settings ▸ Models reuses onboarding §39's ProviderCards/ProviderForm so the two
// surfaces can't drift. Settings-only extras: per-card "used Nh ago", a "Remove
// key…" affordance, the global composer-picker card (gallery view), and the
// per-provider ModelChecklist / read-only model preview (form view).
export function ModelsTab() {
  const [settings, setSettings] = useState<ModelSettings | null>(null);
  const refreshSettings = () => getSettings().then(setSettings).catch(() => setSettings(null));
  const ps = useProviderSetup({ onSaved: refreshSettings });
  useEffect(() => {
    refreshSettings();
  }, []);

  if (!settings) return <div className="text-md text-muted">Loading…</div>;

  const info = ps.info;
  const knownNames = ps.providers.map((p) => p.name);

  if (ps.sel === null) {
    return (
      <div>
        <ProviderCards ps={ps} tp="set" gridClass="grid grid-cols-2 xl:grid-cols-3 gap-2.5" lastUsed />
        <ComposerPickerCard settings={settings} providers={ps.providers} onChanged={refreshSettings} />
        <BackgroundModelCard settings={settings} onChanged={refreshSettings} />
        <EmbeddingModelCard settings={settings} onChanged={refreshSettings} />
      </div>
    );
  }

  return (
    <div>
      <ProviderForm
        ps={ps}
        tp="set"
        footer={
          ps.credentialed ? (
            <button
              className="text-sm text-danger/80 hover:text-danger hover:underline underline-offset-2"
              data-testid="set-remove-key"
              onClick={() => {
                if (window.confirm(`Remove the ${info?.title} key from this computer?`)) ps.removeKey();
              }}
            >
              Remove key…
            </button>
          ) : null
        }
      />

      {ps.sel === "openai" && settings.source === "env" && (
        <p className="text-sm text-muted mt-3 leading-prose">
          A key is set via <code>OPENAI_API_KEY</code> in this server's environment. You can override
          it above; the stored key is used only when the environment variable is absent.
        </p>
      )}

      {info?.configured ? (
        <div className="mt-6">
          <div className={SEC_H + " mb-1.5"}>Models</div>
          <p className="text-sm text-muted mb-2.5 leading-prose">
            Ticked models show in the composer's picker; the one marked Default is what new
            sessions start with.
          </p>
          <ModelChecklist
            provider={ps.sel}
            knownProviders={knownNames}
            suggested={info?.suggested_models || []}
            curated={settings.models}
            defaultModel={settings.model}
            labels={settings.model_labels}
            onChanged={(next) => setSettings((s) => (s ? { ...s, models: next.models, model: next.model } : s))}
          />
        </div>
      ) : (
        // Unconfigured providers still show their curated models as a read-only preview — what a
        // key unlocks is part of deciding to get one at all (owner ask, 2026-07-04).
        (info?.suggested_models?.length || 0) > 0 && (
          <div className="mt-6" data-testid="model-preview">
            <div className={SEC_H + " mb-1.5"}>Included models</div>
            <p className="text-sm text-muted mb-2.5 leading-prose">
              Curated, agent-capable models this provider serves — add your key above to enable them.
            </p>
            <div className="space-y-1">
              {(info?.suggested_models || []).map((m) => {
                const full = ps.sel === "openai" ? m : `${ps.sel}:${m}`;
                return (
                  <div
                    key={m}
                    className="px-2.5 py-1.5 rounded-lg border border-line bg-paper text-md text-muted"
                    title={full}
                  >
                    {settings.model_labels?.[full] || m}
                  </div>
                );
              })}
            </div>
          </div>
        )
      )}
    </div>
  );
}

// The gallery view's "In the composer's picker" card: every curated model across providers,
// with its provider tag. Unticking removes it from the picker; adding happens from a
// provider's card (the ModelChecklist there has the suggested list + free-type add).
export function ComposerPickerCard({
  settings,
  providers,
  onChanged,
}: {
  settings: ModelSettings;
  providers: ProviderInfo[];
  onChanged: () => void;
}) {
  const names = providers.map((p) => p.name);
  const provOf = (id: string) => {
    const i = id.indexOf(":");
    return i > 0 && names.includes(id.slice(0, i)) ? id.slice(0, i) : "openai";
  };
  /** Who serves this model. The server's own label says so — "llama3.2:latest · Ollama · 131k
   *  context" — and that is the only thing that knows: an id with no provider prefix fell back
   *  to "openai", which tagged every model pulled onto this Mac as OpenAI's. */
  const tag = (id: string) => {
    const vendor = settings.model_labels?.[id]?.split(" · ")[1];
    if (vendor) return vendor.split(" — ")[0].split(" (")[0].trim();
    const p = providers.find((x) => x.name === provOf(id));
    return (p?.title || provOf(id)).split(" (")[0];
  };
  return (
    <div className="mt-6" data-testid="composer-picker">
      <div className={SEC_H + " mb-1.5"}>In the composer's picker</div>
      <p className="text-sm text-muted mb-2.5 leading-prose">
        The models offered when starting a session; the one marked Default is what they start
        with. Add more from a provider's card above.
      </p>
      <div className="mlist">
        {settings.models.map((id) => {
          const isDefault = id === settings.model;
          return (
            <div className="mlist-row" key={id}>
              <label className="mlist-main">
                <input
                  type="checkbox"
                  checked
                  disabled={isDefault}
                  title={isDefault ? "The default model is always shown — make another model default first" : "Remove from the picker"}
                  onChange={() => removeModel(id).then((r) => r.ok && onChanged())}
                />
                <span className="mlist-name" title={id}>
                  {settings.model_labels?.[id] || id}
                </span>
              </label>
              <span className="text-xs text-faint mr-2 shrink-0">{tag(id)}</span>
              {isDefault ? (
                <span className="mlist-default">default</span>
              ) : (
                <button className="mlist-make" onClick={() => setDefaultModel(id).then(() => onChanged())}>
                  Make default
                </button>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
