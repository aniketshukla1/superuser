import { shortPersonaName } from "../../personaScope";
import { Icon, ICON } from "../Icon";
import { RemoteAgentSection } from "../RemoteAgentSection";
import { PersonaConnections } from "./PersonaConnections";
import { PersonaDefaults } from "./PersonaDefaults";
import { PersonaDelegation } from "./PersonaDelegation";
import { PersonaSummary } from "./PersonaSummary";
import { usePersonaDetail } from "./usePersonaDetail";

export function PersonaView({
  personaId,
  onBack,
  onOpenIntegrations,
}: {
  personaId: string;
  onBack?: () => void;
  onOpenIntegrations?: () => void;
}) {
  const {
    detail, setDetail, workers, others, byName, error, browserError,
    saveMode, toggleAsk, toggleBrowser, toggleWorker, toggleEnabled, toggleDefault,
  } = usePersonaDetail(personaId);

  const header = (
    <div className="h-12 shrink-0 px-5 flex items-center gap-3 border-b border-line bg-paper">
      {onBack && (
        <>
          <button
            className="inline-flex items-center gap-1 text-sm text-muted hover:text-ink"
            onClick={onBack}
          >
            <Icon name="arrowLeft" size={ICON.md} /> Back
          </button>
          <span className="text-faint">·</span>
        </>
      )}
      <span className="text-md font-semibold">Persona</span>
    </div>
  );

  if (error || !detail) {
    return (
      <main className="flex-1 min-w-0 flex flex-col bg-paper">
        {header}
        <div className="p-12 text-center text-faint text-md">{error || "Loading…"}</div>
      </main>
    );
  }

  return (
    <main className="flex-1 min-w-0 flex flex-col bg-paper">
      {header}
      <div className="flex-1 overflow-y-auto hairline-scroll">
        <div className="max-w-3xl mx-auto px-7 py-6 space-y-6">
          <PersonaSummary detail={detail} personaId={personaId} toggleEnabled={toggleEnabled} />

          <PersonaConnections
            detail={detail}
            personaId={personaId}
            byName={byName}
            toggleDefault={toggleDefault}
            onOpenIntegrations={onOpenIntegrations}
          />

          <PersonaDelegation
            detail={detail}
            personaId={personaId}
            workers={workers}
            others={others}
            browserError={browserError}
            toggleWorker={toggleWorker}
            toggleBrowser={toggleBrowser}
            toggleAsk={toggleAsk}
          />

          {/* Where its turns run: here, or on a remote AG-UI agent. Offered for assistants
              a person made or installed as well as built-ins — any of them may be the face
              of an agent that lives somewhere else. */}
          <RemoteAgentSection
            personaId={personaId}
            name={shortPersonaName(detail.name, personaId)}
            remote={detail.remote}
            tokenSet={!!detail.remote_token_set}
            onSaved={(remote, tokenSet) => setDetail((d) => (d ? { ...d, remote, remote_token_set: tokenSet } : d))}
          />

          <PersonaDefaults detail={detail} saveMode={saveMode} />
        </div>
      </div>
    </main>
  );
}
