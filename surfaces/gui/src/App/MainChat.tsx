import { Icon, ICON } from "../components/Icon";
import { TranscriptNav } from "../components/TranscriptNav";
import { ChatComposer } from "./ChatComposer";
import { ChatScroll } from "./ChatScroll";
import type { AppModel } from "./model/useAppModel";
import { RunBanner } from "./RunBanner";

/** Offered when the reader has scrolled up while the transcript is still growing. A strip of no
 *  height lets the pill float over the foot of the scroll area, above the composer, without
 *  taking room in the layout. */
function JumpToLatest({ onClick }: { onClick: () => void }) {
  return (
    <div className="h-0 relative z-10">
      <button
        className="-translate-x-1/2 absolute bg-panel border border-line bottom-3 cursor-pointer flex gap-1.5 hover:text-ink items-center left-1/2 px-3 py-1.5 rounded-full shadow-md text-muted text-sm whitespace-nowrap"
        data-testid="jump-to-latest"
        onClick={onClick}
      >
        <Icon name="chevronDown" size={ICON.md} />
        Jump to latest
      </button>
    </div>
  );
}

/** The chat column: the run banner, the transcript and the composer. */
export function MainChat({ m }: { m: AppModel }) {
  const growing = m.running || Boolean(m.streaming);
  return (
    <main className="main-chat">
      {/* A run of an automation (a __run__ session) gets a banner back to its runs (owner,
          2026-07-04). It sits INSIDE the chat column, which is padded clear of the glass topbar;
          placed above .main-workspace it ended up under the topbar. */}
      <RunBanner m={m} />
      {/* Kept outside the scroller: inside, it would scroll away with the transcript, the one
          thing a position marker must not do. `.main` is position:relative, so it anchors to the pane. */}
      {!m.idle && <TranscriptNav items={m.items} scrollRef={m.scrollRef} />}
      <ChatScroll m={m} />
      {!m.following && growing && <JumpToLatest onClick={m.followLatest} />}
      <ChatComposer m={m} />
    </main>
  );
}
