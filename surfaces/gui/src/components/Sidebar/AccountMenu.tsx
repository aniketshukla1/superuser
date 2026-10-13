import { menuKeys, AttnBadge } from "./Props";
import { cloudLogin, waitForCloudSignIn, announceCloudChanged, cloudLogout } from "../../api";
import { Icon, ICON } from "../Icon";
import { NAV } from "../../nav";
import { Fragment } from "react";
import type { SidebarModel } from "./model/useSidebarModel";
import { appMenuItem } from "./render/rail";

/** The account menu above the account row. */
export function AccountMenu({ m }: { m: SidebarModel }) {
  const { appMenuOpen, setAppMenuOpen, accountMenuRef, accountRowRef, cloud, accountEmail,
    setCloud, navTarget, totalAttention, props } = m;
  return (
    <>{appMenuOpen && (
            <>
              <div className="fixed inset-0 z-30" onClick={() => setAppMenuOpen(false)} />
              <div
                className="absolute z-40 bottom-full left-0 right-0 mb-1 rounded-xl border border-line bg-panel shadow-2xl py-1"
                data-testid="account-menu"
                role="menu"
                aria-label="Account"
                ref={accountMenuRef}
                onKeyDown={(e) => menuKeys(e, () => setAppMenuOpen(false), accountRowRef.current)}
              >
                {cloud?.signed_in ? (
                  <div
                    className="px-3 py-1.5 mb-1 text-xs text-faint truncate border-b border-line"
                    title={`${accountEmail} · Superuser Cloud`}
                  >
                    {accountEmail} · Superuser Cloud
                  </div>
                ) : (
                  <>
                    {/* Says what is true first — the app is running locally, which is the product
                        working as intended — and offers the cloud as the option it is. The sign-in
                        row was the menu's first item in the accent colour, which read as the
                        thing you were supposed to do, beside a Settings card saying there was no
                        account at all. */}
                    <div className="px-3 py-1.5 text-xs text-faint border-b border-line">
                      Running on this Mac. Superuser Cloud is optional — it only brokers one-click connections.
                    </div>
                    <button
                      role="menuitem"
                      className="w-full flex items-center gap-2.5 px-3 py-1.5 mb-1 text-md text-left text-ink hover:bg-paper"
                      data-testid="account-sign-in"
                      onClick={async () => {
                        setAppMenuOpen(false);
                        // Opens the system browser server-side; completion lands out-of-band,
                        // so poll until it flips (refocusing the window also refetches).
                        await cloudLogin().catch(() => {});
                        waitForCloudSignIn((s) => {
                          if (s) setCloud(s);
                          // Other always-mounted consumers (Settings' telemetry card,
                          // connector panes) refetch on this.
                          if (s?.signed_in) announceCloudChanged();
                        });
                      }}
                    >
                      <Icon name="plug" size={ICON.md} className="shrink-0" /> Sign in to Superuser
                      Cloud
                    </button>
                  </>
                )}
                {/* Every surface, from the same list the rail renders — the menu stays the
                    complete index even as entries are promoted to rows. */}
                {NAV.map((entry) => {
                  const { go, active } = navTarget(entry.id);
                  return (
                    <Fragment key={entry.id}>
                      {appMenuItem(m, 
                        entry.icon,
                        entry.label,
                        go,
                        active,
                        entry.id === "inbox" ? <AttnBadge n={totalAttention} /> : undefined,
                      )}
                    </Fragment>
                  );
                })}
                <div className="h-px bg-line my-1 mx-2" />
                {appMenuItem(m, 
                  "gear",
                  "Settings",
                  props.onManage,
                  false,
                  <span className="text-xs text-faint">⌘ ,</span>,
                )}
                {cloud?.signed_in && (
                  <>
                    <div className="h-px bg-line my-1 mx-2" />
                    {appMenuItem(m, "signOut", "Sign out", async () => {
                      await cloudLogout().catch(() => {});
                      announceCloudChanged();
                    })}
                  </>
                )}
              </div>
            </>
          )}</>
  );
}
