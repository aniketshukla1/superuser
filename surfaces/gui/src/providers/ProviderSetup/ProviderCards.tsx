import { ProviderMark, type ProviderSetupState } from "./ProviderSetupState";

const CARD = "bg-panel border border-line flex gap-2.5 hover:border-lineStrong items-center px-3 py-2.5 rounded-xl text-left transition-colors";

type Props = {
  ps: ProviderSetupState;
  /** Test-id prefix: "ob" in onboarding, "set" in Settings. */
  tp: string;
  gridClass?: string;
  lastUsed?: boolean;
};

/** The gallery: a card per provider, each showing where it stands. */
export function ProviderCards({ ps, tp, gridClass = "gap-2.5 grid grid-cols-2", lastUsed = false }: Props) {
  return (
    <div className={gridClass}>
      {ps.ordered.map((provider) => (
        <button key={provider.name} className={CARD} data-testid={`${tp}-provider-${provider.name}`} onClick={() => ps.openProvider(provider.name)}>
          <ProviderMark name={provider.name} title={provider.title} />
          <span className="flex-1 min-w-0">
            <span className="block font-semibold leading-tight text-md truncate">{provider.title}</span>
            {ps.statusFor(provider, { lastUsed })}
          </span>
          <span className="text-faint text-lg">›</span>
        </button>
      ))}
    </div>
  );
}
