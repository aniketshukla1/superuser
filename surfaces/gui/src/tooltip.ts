// One tooltip for the whole app. Any element with `data-tip` gets a styled label on hover
// (after a short pause) or at once on keyboard focus. It is a single fixed-position node,
// placed against the viewport, so a scrolling sidebar or an `overflow: hidden` row can never
// clip it. Meant for icon-only controls, where the name is not on screen: the control keeps
// its own `aria-label`, so the tooltip is visual only and hidden from screen readers.

const DELAY_MS = 350;
const GAP = 6; // px between the control and the label
const EDGE = 8; // px the label keeps from the window edge

let tip: HTMLDivElement | null = null;
let owner: HTMLElement | null = null;
let timer: number | undefined;

function node(): HTMLDivElement {
  if (!tip) {
    tip = document.createElement("div");
    tip.className = "tip";
    tip.setAttribute("aria-hidden", "true");
    document.body.appendChild(tip);
  }
  return tip;
}

function show(el: HTMLElement) {
  const text = el.dataset.tip;
  if (!text || !el.isConnected) return;
  const t = node();
  t.textContent = text;
  t.classList.remove("is-on");
  // Measure first, then place: below the control, centred, flipped above when the window
  // has no room underneath, and slid sideways to stay inside it.
  const r = el.getBoundingClientRect();
  const w = t.offsetWidth;
  const h = t.offsetHeight;
  const below = r.bottom + GAP + h <= window.innerHeight - EDGE;
  const top = below ? r.bottom + GAP : r.top - GAP - h;
  const left = Math.min(Math.max(r.left + r.width / 2 - w / 2, EDGE), window.innerWidth - EDGE - w);
  t.style.top = `${Math.round(top)}px`;
  t.style.left = `${Math.round(left)}px`;
  t.dataset.side = below ? "below" : "above";
  t.classList.add("is-on");
}

function hide() {
  window.clearTimeout(timer);
  owner = null;
  tip?.classList.remove("is-on");
}

/** The element whose tooltip a pointer or focus is on, if any. Opt-in by `data-tip`: a plain
 * `title` is left to the browser — rows and tests find things by it, so it is never rewritten. */
export function tipTarget(target: EventTarget | null): HTMLElement | null {
  return target instanceof Element ? target.closest<HTMLElement>("[data-tip]") : null;
}
const from = tipTarget;

/** Call once at startup. */
export function initTooltips() {
  document.addEventListener("pointerover", (e) => {
    const el = from(e.target);
    if (el === owner) return;
    hide();
    if (!el) return;
    owner = el;
    timer = window.setTimeout(() => owner === el && show(el), DELAY_MS);
  });
  document.addEventListener("pointerout", (e) => {
    if (owner && !(e.relatedTarget instanceof Node && owner.contains(e.relatedTarget))) hide();
  });
  document.addEventListener("focusin", (e) => {
    const el = from(e.target);
    if (!el || !(e.target as Element).matches(":focus-visible")) return;
    hide();
    owner = el;
    show(el);
  });
  document.addEventListener("focusout", hide);
  // A click, a key or a scroll means the person has moved on.
  document.addEventListener("pointerdown", hide, true);
  document.addEventListener("keydown", hide, true);
  document.addEventListener("scroll", hide, true);
}
