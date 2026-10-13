import { useEffect, useRef, type RefObject } from "react";

/** A modal's focus contract: focus the input on open, give focus back on close,
 * close on Escape and keep Tab inside the panel, and make the app behind inert. */
export function useModalFocus(
  panelRef: RefObject<HTMLDivElement>,
  inputRef: RefObject<HTMLInputElement>,
  onClose: () => void,
) {
  const returnTo = useRef<HTMLElement | null>(null);

  useEffect(() => {
    // Remember who opened us, so closing hands focus back rather than dropping it on
    // <body> (UX-REPORT §6).
    returnTo.current = document.activeElement as HTMLElement | null;
    inputRef.current?.focus();
    return () => returnTo.current?.focus?.();
  }, []);

  // Escape used to live on the overlay's onKeyDown, so it stopped working the moment
  // Tab carried focus out of the palette — and Tab did, because nothing trapped it.
  // A document-level listener means the key works from wherever focus ended up.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key !== "Tab") return;
      const panel = panelRef.current;
      if (!panel) return;
      const focusable = Array.from(
        panel.querySelectorAll<HTMLElement>(
          'button, a[href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
        ),
      ).filter((el) => el.offsetParent !== null || el === document.activeElement);
      if (!focusable.length) return;
      // Every Tab is handled here, rather than only the two edges. WebKit — which is
      // what the desktop shell runs — moves Tab between FORM CONTROLS only unless the
      // user has turned on "Press Tab to highlight each item", so a trap that guards
      // `here === last` never fires: Tab leaves the sole input, skips every button in
      // the list, and lands on <body>. Driving the cycle ourselves is the only version
      // that holds on both engines (found by the webkit project in
      // playwright.tauri.config.ts).
      e.preventDefault();
      const here = document.activeElement as HTMLElement | null;
      const at = here ? focusable.indexOf(here) : -1;
      const next = at === -1
        ? e.shiftKey ? focusable.length - 1 : 0
        : (at + (e.shiftKey ? -1 : 1) + focusable.length) % focusable.length;
      focusable[next].focus();
    };
    document.addEventListener("keydown", onKeyDown, true);
    return () => document.removeEventListener("keydown", onKeyDown, true);
  }, [onClose]);

  // Take the app behind out of the accessibility tree and out of the tab order for as
  // long as the palette is up. The palette itself is portalled to <body> so it is not a
  // descendant of the thing being made inert.
  useEffect(() => {
    const shell = document.querySelector<HTMLElement>(".app");
    if (!shell) return;
    shell.setAttribute("inert", "");
    shell.setAttribute("aria-hidden", "true");
    return () => {
      shell.removeAttribute("inert");
      shell.removeAttribute("aria-hidden");
    };
  }, []);
}
