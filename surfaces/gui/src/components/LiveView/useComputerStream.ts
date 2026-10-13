import { useEffect, useRef, useState } from "react";
import { getComputer, openComputerSocket, type ComputerControl, type ComputerStatus } from "../../api";

/** The assistant's browser as it streams in: frames, who holds control, where it is. */
export function useComputerStream(persona: string) {
  const [frame, setFrame] = useState<{ src: string; width: number; height: number } | null>(null);
  const [control, setControl] = useState<ComputerControl | null>(null);
  const [status, setStatus] = useState<ComputerStatus | null>(null);
  const [stopped, setStopped] = useState<string | null>(null);
  const [notice, setNotice] = useState("");
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    const ws = openComputerSocket(persona);
    wsRef.current = ws;
    ws.onmessage = (e) => {
      let m: any;
      try {
        m = JSON.parse(String(e.data));
      } catch {
        return;
      }
      if (m.type === "frame" && typeof m.data === "string") {
        setFrame({ src: `data:image/jpeg;base64,${m.data}`, width: m.width || 1280, height: m.height || 800 });
        setStopped(null);
      } else if (m.type === "stopped") {
        setStopped(String(m.reason || "The browser closed."));
      } else if (m.type === "control") {
        const { type: _type, ...rest } = m;
        setControl(rest as ComputerControl);
      } else if (m.type === "error") {
        setNotice(String(m.message || ""));
      }
    };
    ws.onclose = () => {
      if (wsRef.current === ws) wsRef.current = null;
    };
    return () => ws.close();
  }, [persona]);

  // Where it is — the frames carry pixels, not addresses.
  useEffect(() => {
    let live = true;
    const load = () =>
      getComputer(persona)
        .then((s) => live && s && setStatus(s))
        .catch(() => {});
    load();
    const t = window.setInterval(load, 2000);
    return () => {
      live = false;
      clearInterval(t);
    };
  }, [persona]);

  return { frame, control, setControl, status, stopped, notice, setNotice, wsRef };
}
