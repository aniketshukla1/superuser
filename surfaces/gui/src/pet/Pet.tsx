import { useEffect, useRef, useState } from "react";
import { isTauri, showMainWindow, dragPetWindow, onTauriEvent } from "../tauri";
import TermPet, { MascotMood } from "./TermPet";

// Superuser's desktop pet. The Rust shell (lib.rs) runs this in a transparent,
// always-on-top window (index.html?mascot), shows it when the main app is
// minimized, and pushes notifications through the `mascot-message` event. The pet
// shows them in a speech bubble (and reads them aloud if you opt into voice);
// clicking it re-opens the app, dragging moves the window.

// Ambient "passing thoughts" while the app is away, until a real message arrives.
const QUIPS = [
  "Indexed 3 new memories ✨",
  "Everything's synced — and local 🔒",
  "Reminder: standup in 15 min ⏰",
  "Sarah replied in Slack 💬",
  "Click me to open Superuser 👆",
];

// Voice is opt-in (localStorage "steward-pet-voice" === "on") so the pet never
// surprises you with audio; bubbles always show.
const voiceOn = () => {
  try {
    return localStorage.getItem("steward-pet-voice") === "on";
  } catch {
    return false;
  }
};

export default function Pet() {
  const [bubble, setBubble] = useState<string | null>(null);
  const [mood, setMood] = useState<MascotMood>("idle");
  const downPos = useRef<{ x: number; y: number } | null>(null);
  const dragged = useRef(false);
  const timers = useRef<number[]>([]);
  // Always running but hidden behind the app; only react to messages when on screen.
  const onScreen = useRef(!isTauri());

  const say = (msg: string) => {
    if (!onScreen.current) return;
    timers.current.forEach(clearTimeout);
    setMood("talking");
    setBubble(msg);
    if (voiceOn()) {
      try {
        window.speechSynthesis?.speak(new SpeechSynthesisUtterance(msg.replace(/[^\p{L}\p{N} .,!?'-]/gu, "")));
      } catch {
        /* no speech synthesis — bubble is enough */
      }
    }
    timers.current = [
      window.setTimeout(() => setMood("idle"), 2200),
      window.setTimeout(() => setBubble(null), 5200),
    ];
  };

  // The Rust watcher tells us when we're actually on screen (app minimized).
  useEffect(() => {
    let un = () => {};
    onTauriEvent<boolean>("mascot-visible", (visible) => {
      onScreen.current = visible;
      if (!visible) {
        window.speechSynthesis?.cancel();
        setBubble(null);
        setMood("idle");
      }
    }).then((f) => (un = f));
    return () => un();
  }, []);

  // Real notifications from the app (payload may be JSON {message,type}).
  useEffect(() => {
    let un = () => {};
    onTauriEvent<string>("mascot-message", (payload) => {
      try {
        const p = JSON.parse(payload);
        say(p.message ?? payload);
      } catch {
        say(payload);
      }
    }).then((f) => (un = f));
    return () => un();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Ambient quips so it feels alive while you're away.
  useEffect(() => {
    const t = setInterval(() => {
      if (Math.random() < 0.6) say(QUIPS[Math.floor(Math.random() * QUIPS.length)]);
    }, 11000);
    return () => clearInterval(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const openApp = () => {
    setMood("happy");
    showMainWindow();
    window.setTimeout(() => setMood("idle"), 1200);
  };

  // Distinguish a click (open) from a drag (move the window).
  const onDown = (e: React.MouseEvent) => {
    if (e.button !== 0) return;
    downPos.current = { x: e.clientX, y: e.clientY };
    dragged.current = false;
  };
  const onMove = (e: React.MouseEvent) => {
    if (!downPos.current || dragged.current) return;
    if (Math.hypot(e.clientX - downPos.current.x, e.clientY - downPos.current.y) > 4) {
      dragged.current = true;
      dragPetWindow();
    }
  };
  const onUp = () => {
    if (downPos.current && !dragged.current) openApp();
    downPos.current = null;
  };

  return (
    <div className="mascot-root">
      {bubble && <div className="mascot-bubble">{bubble}</div>}
      <div
        className={"mascot-orb" + (mood === "happy" ? " is-happy" : "")}
        onMouseDown={onDown}
        onMouseMove={onMove}
        onMouseUp={onUp}
        title="Click to open Superuser · drag to move"
      >
        <TermPet size={128} mood={mood} />
      </div>
    </div>
  );
}
