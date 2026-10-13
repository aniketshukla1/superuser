import { TranscriptNav } from "../components/TranscriptNav";
import { Icon, ICON } from "../components/Icon";
import type { AppModel } from "./model/useAppModel";
import { RunBanner } from "./RunBanner";
import { ChatScroll } from "./ChatScroll";
import { ChatComposer } from "./ChatComposer";

/** The chat column: run banner, transcript and composer. */
export function MainChat({ m }: { m: AppModel }) {
  const { idle, items, scrollRef, following, running, streaming, followLatest } = m;
  return (
    <main className="main-chat">
            {/* Automation-run context (owner ask 2026-07-04): a __run__ session looked like any
                other chat with no way back to the runs list. Lives INSIDE the chat column (which
                is padded to clear the absolute glass topbar — rendering above .main-workspace put
                it underneath the topbar; owner-reported CSS bug). */}
            <RunBanner m={m} />
            {/* Outside the scroller on purpose: inside, it scrolls away with the transcript,
                which is the one thing a position marker must not do. `.main` is already
                position:relative, so it anchors against the pane. */}
            {!idle && <TranscriptNav items={items} scrollRef={scrollRef} />}
            <ChatScroll m={m} />

            {/* Scrolled up while the transcript is still growing → offer the way back down.
                Zero-height strip keeps the pill floating over the scroll area, above the
                composer, without reserving layout space. */}
            {!following && (running || !!streaming) && (
              <div className="relative h-0 z-10">
                <button
                  className="absolute bottom-3 left-1/2 -translate-x-1/2 flex items-center gap-1.5 px-3 py-1.5 rounded-full border border-line bg-panel shadow-md text-sm text-muted hover:text-ink cursor-pointer whitespace-nowrap"
                  data-testid="jump-to-latest"
                  onClick={followLatest}
                >
                  <Icon name="chevronDown" size={ICON.md} />
                  Jump to latest
                </button>
              </div>
            )}

            <ChatComposer m={m} />
                  </main>
  );
}
