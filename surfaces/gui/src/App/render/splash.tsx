import { LaunchFilm } from "../../components/LaunchFilm";
import type { AppModel } from "../model/useAppModel";

  // The splash, first on its own and then laid over the app while it plays its exit. It
  // sits at the same place in the tree in both returns below (second child of a
  // fragment), so React keeps the one instance and its intro never replays.
  export function renderSplash(m: AppModel) {
  const { splashGone, overlay, splashLeaving, appReady, reelDone, introSkipped, simOverlay,
      resumedExisting, bootFailed, retryBoot, healthOk, intro, setIntroSkipped,
      setReelDone, setSplashGone, personas, booting, connected } = m;
  return !splashGone && (
    <div
      className={"boot-splash" + (overlay ? " tauri-overlay" : "") + (splashLeaving ? " boot-leaving" : "")}
      // Decoration on its way out once the app is up and the reel is over. Not while the
      // reel still plays over a ready app: its Skip control has to stay reachable.
      aria-hidden={(appReady && (reelDone || introSkipped)) || undefined}
    >
      {/* The drag region only: the splash carries its own wordmark, so the titlebar's
          would say the same thing twice. */}
      {overlay && <div className="titlebar-drag" data-tauri-drag-region />}
      {simOverlay && (
        <div className="sim-traffic-lights" aria-hidden="true">
          <span /><span /><span />
        </div>
      )}
      {/* Each node is wired to the signal it names; see UX-REPORT §2. */}
      <LaunchFilm
        failed={bootFailed}
        onRetry={retryBoot}
        intro={intro}
        skipped={introSkipped}
        onSkip={() => setIntroSkipped(true)}
        onReelEnd={() => setReelDone(true)}
        leaving={splashLeaving}
        onLeft={() => setSplashGone(true)}
        steps={[
          {
            label: "Local runtime",
            state: healthOk ? true : bootFailed ? "failed" : false,
          },
          {
            label: personas ? `${personas.length} assistant${personas.length === 1 ? "" : "s"}` : "Assistants",
            state: personas !== null ? true : bootFailed ? "failed" : false,
          },
          {
            label: resumedExisting ? "Your last chat" : "New chat",
            state: !booting ? true : bootFailed ? "failed" : false,
          },
          {
            label: "Live connection",
            state: connected ? true : bootFailed ? "failed" : false,
          },
        ]}
      />
    </div>
  );

}
