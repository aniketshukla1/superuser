import { useState, useRef, useEffect } from "react";
import { type CloudStatus, getCloudStatus, CLOUD_CHANGED, INBOX_UNLOCK } from "../../../api";

/** The account row's menu, the search palette and new-assistant modal, and cloud sign-in status. */
export function useAccountMenu() {
  const [searchModalOpen, setSearchModalOpen] = useState(false);
  const [creatingAssistant, setCreatingAssistant] = useState(false);
  const [appMenuOpen, setAppMenuOpen] = useState(false);
  // Escape closes the account menu and hands focus back to the row that opened it.
  // A popup that only the mouse can dismiss (its scrim was the sole way out) strands
  // a keyboard user in a menu they can't leave — same defect the search palette had.
  const accountRowRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!appMenuOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      setAppMenuOpen(false);
      accountRowRef.current?.focus();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [appMenuOpen]);
  // And opening it puts focus on its first item, so the arrow keys work straight away.
  const accountMenuRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (appMenuOpen) accountMenuRef.current?.querySelector<HTMLElement>("[role=menuitem]")?.focus();
  }, [appMenuOpen]);
  // The account row (§26): cloud sign-in status drives the avatar/name/dot; refreshed on
  // focus and whenever the menu opens (sign-in completes out-of-band in the browser).
  const [cloud, setCloud] = useState<CloudStatus | null>(null);
  // Inbox chip sticky unlock (§26): absent until the product first parks an item (or a
  // session first goes Unattended), then permanent. Per-device, like nav collapse.
  const [inboxUnlocked, setInboxUnlocked] = useState(
    () => localStorage.getItem("ocw:inbox-unlocked") === "1",
  );
  const refreshCloud = () => getCloudStatus().then(setCloud).catch(() => {});
  useEffect(() => {
    refreshCloud();
    const onFocus = () => refreshCloud();
    window.addEventListener("focus", onFocus);
    window.addEventListener(CLOUD_CHANGED, onFocus);
    const unlock = () => {
      localStorage.setItem("ocw:inbox-unlocked", "1");
      setInboxUnlocked(true);
    };
    window.addEventListener(INBOX_UNLOCK, unlock);
    return () => {
      window.removeEventListener("focus", onFocus);
      window.removeEventListener(CLOUD_CHANGED, onFocus);
      window.removeEventListener(INBOX_UNLOCK, unlock);
    };
  }, []);

  return { searchModalOpen, setSearchModalOpen, creatingAssistant, setCreatingAssistant, appMenuOpen, setAppMenuOpen, accountRowRef, accountMenuRef, cloud, setCloud, inboxUnlocked, setInboxUnlocked, refreshCloud };
}
