import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import type { BootStep, IntroMode } from "./intro";
import { Capture } from "./Capture";
import { Feeds, type Lock } from "./Feeds";
import { FilmBottom } from "./FilmBottom";
import { Converge, Mark } from "./Finale";
import { Name, Tagline } from "./Name";
import { ACTS, FILM_MS, feedPlan, rng } from "./script";
import { Search, type Point } from "./Search";
import { Status } from "./Status";
import { Command, Terminal, grainUrl } from "./Terminal";
import { terminalLines } from "./work";

const SEED = 1979;

/**
 * The launch film (the owner's brief, 2026-10-13). In the dark a command types itself. The camera flies forward through
 * screens of Superuser at work — code being written, a reply drafted, reports drawing
 * themselves, a board, a ledger, a test run — while terminals run down both edges. The
 * machine locks onto work as it gets done; one finished task is brought forward and
 * captured; a scan crosses, every line runs home, the mark draws itself, and the name
 * rolls in and glows on matte black.
 *
 * What is not staged is the bottom bar: each reading is wired to the signal it names, and
 * the app opens when those are true, not when the film ends; if the runtime never answers,
 * the last frame says so and offers a retry. The
 * first launch plays the whole film (~11s), later ones a five-second cut; Skip or Escape
 * goes straight to the app whenever it is ready; reduced motion, or the intro switched
 * off, shows the last frame only. Same props as the reel it replaces.
 */
export function LaunchFilm({
  steps,
  failed,
  onRetry,
  intro = "off",
  skipped,
  onSkip,
  onReelEnd,
  leaving,
  onLeft,
}: {
  steps: BootStep[];
  failed?: boolean;
  onRetry?: () => void;
  intro?: IntroMode;
  skipped?: boolean;
  onSkip?: () => void;
  onReelEnd?: () => void;
  leaving?: boolean;
  onLeft?: () => void;
}) {
  const cut = intro === "off" ? null : intro;
  const playing = cut !== null && !skipped;
  const acts = ACTS[cut ?? "short"];
  const [ended, setEnded] = useState(intro === "off");
  const born = useRef(performance.now());

  useEffect(() => {
    if (!cut) return;
    const t = window.setTimeout(() => {
      setEnded(true);
      onReelEnd?.();
    }, FILM_MS[cut]);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Escape skips. Not Enter or Space: the composer may already hold focus underneath.
  useEffect(() => {
    if (!playing || ended) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onSkip?.();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [playing, ended, onSkip]);

  // The exit. `animationend` bubbles from every child, so only the root's own counts; the
  // timer covers a window that never paints the end (hidden, throttled).
  const left = useRef(false);
  const finish = () => {
    if (left.current) return;
    left.current = true;
    onLeft?.();
  };
  useEffect(() => {
    if (!leaving) return;
    const t = window.setTimeout(finish, 900);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [leaving]);

  const plan = useMemo(() => (cut ? feedPlan(cut, SEED) : []), [cut]);
  const lines = useMemo(() => {
    const r = rng(SEED + 1);
    return [terminalLines(r, 90), terminalLines(r, 90)];
  }, []);
  const grain = useMemo(() => grainUrl(), []);

  const [locks, setLocks] = useState<Map<string, Lock & { point: Point }>>(new Map());
  const onLock = useCallback(
    (key: string, point: Point) => {
      const verdict = plan.find((f) => f.key === key)?.task.verdict ?? "done";
      setLocks((prev) => new Map(prev).set(key, { at: performance.now() - born.current, verdict, point }));
    },
    [plan],
  );
  const lockPoints = useRef<Point[]>([]);
  lockPoints.current = [...locks.values()].map((l) => l.point);

  // Where everything runs home to: the middle of the mark.
  const markBox = useRef<HTMLDivElement>(null);
  const centre = useCallback((): Point => {
    const box = markBox.current?.getBoundingClientRect();
    return box && box.width > 0
      ? { x: box.left + box.width / 2, y: box.top + box.height / 2 }
      : { x: window.innerWidth / 2, y: window.innerHeight * 0.4 };
  }, []);

  const clock = {
    "--feeds": `${acts.feeds}ms`,
    "--sweep": `${acts.sweep}ms`,
    "--cam": `${acts.sweep - acts.feeds + 300}ms`,
    "--mark": `${acts.mark}ms`,
    "--cmd-out": `${acts.feeds + 250}ms`,
  } as CSSProperties;

  return (
    <div
      className={`boot-term film reel-${playing ? cut : "still"}${failed ? " is-failed" : ""}${leaving ? " is-leaving" : ""}`}
      style={clock}
      role="status"
      aria-live="polite"
      aria-label="Starting Superuser"
      onAnimationEnd={(e) => {
        if (leaving && e.target === e.currentTarget) finish();
      }}
    >
      {playing && cut && (
        <>
          <div className="film-term" aria-hidden="true">
            <Terminal lines={lines[0]} side="left" seconds={7.4} />
            <Terminal lines={lines[1]} side="right" seconds={5.6} />
          </div>
          <div className="film-footage">
            <Feeds plan={plan} locks={locks} />
            <Search cut={cut} plan={plan} onLock={onLock} centre={centre} />
          </div>
          <Capture acts={acts} />
          <div className="film-sweep" aria-hidden="true" />
          <Converge at={acts.converge} from={() => lockPoints.current} centre={centre} />
          {acts.command >= 0 && <Command at={acts.command} />}
          {grain && <div className="film-grain" style={{ backgroundImage: `url(${grain})` }} aria-hidden="true" />}
        </>
      )}
      <div className="film-flare" aria-hidden="true" />
      <div className="film-scanlines" aria-hidden="true" />
      <div className="film-vignette" aria-hidden="true" />

      <div className="film-final">
        <div ref={markBox} className="film-mark-box">
          <Mark at={acts.mark} still={!playing} />
        </div>
        <Name at={acts.title} each={cut === "full" ? 85 : 55} glow={acts.glow} still={!playing} />
        <Tagline at={acts.tagline} still={!playing} />
        <Status failed={failed} onRetry={onRetry} />
      </div>
      <FilmBottom steps={steps} showSkip={playing && !ended} onSkip={onSkip} />
    </div>
  );
}
