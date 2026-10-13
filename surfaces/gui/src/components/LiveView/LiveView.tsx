import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { computerControl } from "../../api";
import { SecretCard } from "../ComputerCards";
import { Icon, ICON } from "../Icon";
import { BTN_ACCENT, BTN_BORDERED, BUTTONS, modifiers } from "./BTN_ACCENT";
import { LiveViewFooter } from "./LiveViewFooter";
import { useComputerStream } from "./useComputerStream";

export function LiveView({ persona, name, onClose }: { persona: string; name?: string; onClose: () => void }) {
  const who = name || "The assistant";
  const { frame, control, setControl, status, stopped, notice, setNotice, wsRef } = useComputerStream(persona);
  const [closing, setClosing] = useState(false);
  const [busy, setBusy] = useState(false);
  const screenRef = useRef<HTMLDivElement>(null);
  const lastMove = useRef(0);

  const driving = control?.holder === "user";

  const send = useCallback(
    (m: Record<string, unknown>) => {
      if (!driving) return;
      const ws = wsRef.current;
      if (ws && ws.readyState === WebSocket.OPEN) ws.send(JSON.stringify(m));
    },
    [driving],
  );

  // The picture is scaled to fit; the page wants its own pixels.
  const at = (e: { clientX: number; clientY: number }) => {
    const box = screenRef.current?.getBoundingClientRect();
    if (!box || !frame) return { x: 0, y: 0 };
    return {
      x: Math.round(((e.clientX - box.left) / box.width) * frame.width),
      y: Math.round(((e.clientY - box.top) / box.height) * frame.height),
    };
  };

  // Registered by hand: React's wheel listener is passive, so it could not keep the modal
  // itself from scrolling while the page scrolls.
  useEffect(() => {
    const el = screenRef.current;
    if (!el || !driving) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      send({ type: "wheel", ...at(e), deltaX: e.deltaX, deltaY: e.deltaY, modifiers: modifiers(e) });
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [driving, send, frame]);

  const change = async (action: "take" | "release") => {
    setBusy(true);
    const res = await computerControl(persona, action).catch(() => null);
    setBusy(false);
    if (!res?.ok) {
      setNotice(res?.error || "That didn't reach the browser.");
      return false;
    }
    if (res.control) setControl(res.control);
    if (action === "take") window.setTimeout(() => screenRef.current?.focus(), 30);
    return true;
  };

  const close = () => (driving ? setClosing(true) : onClose());

  const secret = control?.secret_wanted;

  return createPortal(
    <div
      className="fixed inset-0 z-50"
      role="dialog"
      aria-modal="true"
      aria-label={`${who}'s browser`}
      data-testid="live-view"
      onKeyDown={(e) => {
        if (e.key === "Escape" && !driving) {
          e.stopPropagation();
          close();
        }
      }}
    >
      <div className="absolute inset-0 bg-black/40" onClick={close} />
      <div className="absolute left-1/2 top-[4vh] -translate-x-1/2 w-[1100px] max-w-[94vw] max-h-[92vh] flex flex-col rounded-xl2 border border-line bg-panel shadow-2xl overflow-hidden">
        <div className="flex items-center gap-2.5 px-4 py-2.5 border-b border-line">
          <Icon name="globe" size={ICON.md} className="text-faint shrink-0" />
          <div className="min-w-0 flex-1">
            <div className="text-md font-medium text-ink truncate">
              {who}&rsquo;s browser
              {status?.title && <span className="text-muted font-normal"> — {status.title}</span>}
            </div>
            {status?.url && <div className="text-sm text-faint truncate">{status.url}</div>}
          </div>
          <span className={"text-sm shrink-0 " + (driving ? "text-accent font-medium" : "text-muted")} data-testid="live-view-holder">
            {driving ? "You have control" : `${who} has control`}
          </span>
          {driving ? (
            <button className={BTN_ACCENT} disabled={busy} onClick={() => void change("release")} data-testid="live-view-release">
              Hand back
            </button>
          ) : (
            <button className={BTN_BORDERED} disabled={busy} onClick={() => void change("take")} data-testid="live-view-take">
              Take control
            </button>
          )}
          <button className="nav-icon-btn" aria-label="Close" onClick={close} data-testid="live-view-close">
            <Icon name="x" size={ICON.md} />
          </button>
        </div>

        {control?.requested && !driving && (
          <div className="px-4 py-2 border-b border-line bg-accentSoft text-sm text-ink" data-testid="live-view-ask">
            <b className="font-medium">{who} needs you:</b> {control.reason || "take control to continue."}
          </div>
        )}
        {secret && (
          <div className="px-4 py-3 border-b border-line">
            <SecretCard persona={persona} name={who} label={secret} onDone={() => {}} compact />
          </div>
        )}

        <div className="flex-1 min-h-0 overflow-auto bg-paper p-3">
          {frame ? (
            <div
              ref={screenRef}
              tabIndex={driving ? 0 : -1}
              className={
                "relative mx-auto outline-none rounded-lg overflow-hidden border " +
                (driving ? "border-accent cursor-default" : "border-line cursor-not-allowed")
              }
              style={{ aspectRatio: `${frame.width} / ${frame.height}`, maxWidth: "100%" }}
              data-testid="live-view-screen"
              onMouseDown={(e) => {
                if (!driving) return;
                screenRef.current?.focus();
                send({ type: "mouse", event: "pressed", ...at(e), button: BUTTONS[e.button] ?? "left", clickCount: e.detail || 1, modifiers: modifiers(e) });
              }}
              onMouseUp={(e) =>
                send({ type: "mouse", event: "released", ...at(e), button: BUTTONS[e.button] ?? "left", clickCount: e.detail || 1, modifiers: modifiers(e) })
              }
              onMouseMove={(e) => {
                const now = performance.now();
                if (now - lastMove.current < 33) return;
                lastMove.current = now;
                send({ type: "mouse", event: "moved", ...at(e), modifiers: modifiers(e) });
              }}
              onContextMenu={(e) => driving && e.preventDefault()}
              onKeyDown={(e) => {
                if (!driving) return;
                // A paste arrives as its own event, with the text; the keystroke would
                // only reach a page that has no clipboard.
                if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "v") return;
                e.preventDefault();
                e.stopPropagation();
                const printable = e.key.length === 1 && !e.ctrlKey && !e.metaKey;
                send({ type: "key", event: "down", key: e.key, code: e.code, ...(printable ? { text: e.key } : {}), modifiers: modifiers(e) });
              }}
              onKeyUp={(e) => {
                if (!driving) return;
                e.preventDefault();
                send({ type: "key", event: "up", key: e.key, code: e.code, modifiers: modifiers(e) });
              }}
              onPaste={(e) => {
                if (!driving) return;
                e.preventDefault();
                const text = e.clipboardData.getData("text");
                if (text) send({ type: "text", text });
              }}
            >
              <img src={frame.src} alt="" draggable={false} className="block w-full h-auto select-none pointer-events-none" />
              {stopped && (
                <div className="absolute inset-0 grid place-items-center bg-black/40">
                  <span className="rounded-lg border border-line bg-panel px-3 py-2 text-md text-ink">{stopped}</span>
                </div>
              )}
            </div>
          ) : (
            <div className="aspect-[16/10] grid place-items-center text-sm text-faint text-center px-6">
              {stopped
                ? stopped
                : status && !status.running
                  ? `${who}'s browser isn't open. It appears here as soon as ${who} opens a page.`
                  : "Waiting for the screen…"}
            </div>
          )}
        </div>

        {(notice || closing || !driving) && (
          <LiveViewFooter who={who} closing={closing} notice={notice} busy={busy} onRelease={() => change("release")} onClose={onClose} />
        )}
      </div>
    </div>,
    document.body,
  );
}
