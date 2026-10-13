import { initials } from "./Props";
import { Icon, ICON } from "../Icon";
import type { SidebarModel } from "./model/useSidebarModel";

/** The account row: who is signed in, the Inbox chip, and settings. */
export function AccountRow({ m }: { m: SidebarModel }) {
  const { appMenuOpen, accountRowRef, refreshCloud, setAppMenuOpen, cloud, accountEmail,
    accountName, inboxUnlocked, totalAttention, props } = m;
  return (
    <div className="acct flex items-center gap-1">
            <button
              className={
                "flex-1 min-w-0 flex items-center gap-2 px-2 py-1 rounded-row text-sm text-left " +
                (appMenuOpen ? "bg-line text-ink" : "hover:bg-solid")
              }
              data-testid="account-row"
              ref={accountRowRef}
              onClick={() => {
                if (!appMenuOpen) refreshCloud();
                setAppMenuOpen((v) => !v);
              }}
              onKeyDown={(e) => {
                if (e.key === "ArrowUp" && !appMenuOpen) {
                  e.preventDefault();
                  refreshCloud();
                  setAppMenuOpen(true);
                }
              }}
              aria-haspopup="menu"
              aria-expanded={appMenuOpen}
              aria-label={cloud?.signed_in ? `Account: ${accountEmail}` : "Account: not signed in"}
              data-tip={cloud?.signed_in ? accountEmail : "Everything runs on this Mac. Sign in only for one-click connections."}
            >
              <span
                className={
                  "w-[18px] h-[18px] rounded-full grid place-items-center text-xs font-semibold shrink-0 " +
                  (cloud?.signed_in
                    ? "bg-accentSoft text-accent"
                    : "bg-paper text-faint border border-line")
                }
                aria-hidden
              >
                {cloud?.signed_in ? initials(accountName) : "?"}
              </span>
              <span className={"truncate " + (cloud?.signed_in ? "" : "text-muted")}>
                {cloud?.signed_in ? accountName : "Not signed in"}
              </span>
              {/* Running without the cloud account is the product working as intended,
                  not a setting left undone — "Not signed in" on its own reads as the
                  latter. The marker says what IS true rather than what isn't, in the
                  mono face the rest of the app uses for machine facts. The sign-in
                  affordance is untouched; this only stops its absence reading as a
                  fault. In a narrow sidebar the marker steps aside (a container query in
                  base-sidebar.css) so "Not signed in" is never clipped; the button's
                  tooltip says the same thing in words. */}
              {!cloud?.signed_in && (
                <span className="acct-local font-mono text-xs text-faint shrink-0">
                  local
                </span>
              )}
              {cloud?.signed_in && (
                <span
                  className="w-[7px] h-[7px] rounded-full bg-ok shrink-0"
                  title="Signed in to Superuser Cloud"
                  aria-hidden
                />
              )}
            </button>
            {inboxUnlocked && (
              <button
                className={
                  // min-h-6: it is a real click target (it opens the Inbox), so it
                  // meets the 24px floor rather than relying on the row behind it.
                  "inline-flex items-center gap-1 rounded-full px-2 py-0.5 min-h-6 text-xs shrink-0 " +
                  (totalAttention > 0
                    ? "bg-accentSoft text-accent font-semibold"
                    : "text-faint hover:text-ink hover:bg-solid")
                }
                data-testid="inbox-chip"
                aria-label={
                  totalAttention > 0 ? `Inbox — ${totalAttention} items need you` : "Inbox"
                }
                title={totalAttention > 0 ? `Inbox — ${totalAttention} items need you` : "Inbox"}
                onClick={() => {
                  // The chip goes STRAIGHT to Inbox — the menu is the row's target, not the chip's.
                  setAppMenuOpen(false);
                  props.onOpenInbox();
                }}
              >
                <Icon name="inbox" size={ICON.md} />
                {totalAttention > 0 ? totalAttention : null}
              </button>
            )}
            <button
              className="nav-icon-btn shrink-0"
              data-testid="settings-button"
              data-tip="Settings (⌘,)"
              aria-label="Settings"
              onClick={() => {
                setAppMenuOpen(false);
                props.onManage();
              }}
            >
              <Icon name="gear" size={ICON.md} />
            </button>
          </div>
  );
}
