import type { ReactElement } from "react";
import type { IconName } from "./IconName";
import type { SvgProps } from "./svgProps";

/** Glyphs "clipboard" … "gear". */
export function glyphs2(name: IconName, s: SvgProps): ReactElement | null {
  switch (name) {
    case "clipboard":
      // A clipboard with a short list on it: the day's items, sorted by what matters.
      return (
        <svg {...s}>
          <rect x="6" y="4.5" width="12" height="16.5" rx="2" />
          <path d="M9.5 3h5v3h-5z" />
          <path d="M9 11h6M9 14.5h6M9 18h3.5" />
        </svg>
      );
    case "gauge":
      // A speedometer: the arc of the dial, its ticks, and the needle swung high. The
      // Product Performance assistant's — how fast, and whether it got slower.
      return (
        <svg {...s}>
          <path d="M4.5 17a8.5 8.5 0 1 1 15 0" />
          <path d="M12 6.5V8M6.6 9.1l1 1M17.4 9.1l-1 1M4.6 14h1.5M17.9 14h1.5" />
          <path d="M12 15.5l3.5-4.5" />
          <circle cx="12" cy="15.5" r="1.2" />
        </svg>
      );
    case "bug":
      // A beetle seen from above: a rounded body split down the back, two feelers and
      // three legs a side. The Bug Reproduction assistant's.
      return (
        <svg {...s}>
          <rect x="8" y="7" width="8" height="12" rx="4" />
          <path d="M12 11v8M10 7.5 8.5 4.5M14 7.5l1.5-3M8 11H4.5M16 11h3.5M8 14.5H5M16 14.5h3M8.5 17.5 6 19.5M15.5 17.5 18 19.5" />
        </svg>
      );
    case "receipt":
      // A till receipt: torn along the foot, three lines of print. Money that has been
      // spent, where coins is money that is kept.
      return (
        <svg {...s}>
          <path d="M6 3h12v18l-2-1.5-2 1.5-2-1.5-2 1.5-2-1.5L6 21z" />
          <path d="M9 8h6M9 11.5h6M9 15h3.5" />
        </svg>
      );
    case "mic":
      return (
        <svg {...s}>
          <rect x="8.5" y="3.5" width="7" height="11" rx="3.5" />
          <path d="M5.5 11.5a6.5 6.5 0 0 0 13 0M12 18v3.5M8.5 21.5h7" />
        </svg>
      );
    case "stop":
      return (
        <svg {...s} fill="currentColor" stroke="none">
          <rect x="6.5" y="6.5" width="11" height="11" rx="1.5" />
        </svg>
      );
    case "x":
      return (
        <svg {...s}>
          <path d="M6 6l12 12M18 6L6 18" />
        </svg>
      );
    case "alert":
      // A failure the user can do something about. A triangle rather than a circle, because
      // the circle is already `helpCircle` and the two must not be confusable at 15px.
      return (
        <svg {...s}>
          <path d="M10.3 4.1 2.6 17.5A2 2 0 0 0 4.3 20.5h15.4a2 2 0 0 0 1.7-3L13.7 4.1a2 2 0 0 0-3.4 0z" />
          <path d="M12 9.5v4" />
          <path d="M12 17h.01" />
        </svg>
      );
    case "thumbUp":
      // The verdict buttons were the arrow glyphs ↑ and ↓, which read as scroll or vote
      // controls. A thumb is the one mark nobody has to learn.
      return (
        <svg {...s}>
          <path d="M7 10v10.5H4.5A1.5 1.5 0 0 1 3 19v-7.5A1.5 1.5 0 0 1 4.5 10H7z" />
          <path d="M7 10l3.6-6.3a1.9 1.9 0 0 1 3.5 1.2L13.5 9h5.3a2 2 0 0 1 2 2.4l-1.4 7.3a2.2 2.2 0 0 1-2.2 1.8H7" />
        </svg>
      );
    case "thumbDown":
      return (
        <svg {...s}>
          <path d="M17 14V3.5h2.5A1.5 1.5 0 0 1 21 5v7.5a1.5 1.5 0 0 1-1.5 1.5H17z" />
          <path d="M17 14l-3.6 6.3a1.9 1.9 0 0 1-3.5-1.2l.6-4.1H5.2a2 2 0 0 1-2-2.4l1.4-7.3A2.2 2.2 0 0 1 6.8 3.5H17" />
        </svg>
      );
    case "folderPlus":
      // Same clean folder body + a centered plus (Lucide-style).
      return (
        <svg {...s}>
          <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9l-.81-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h8.5" />
          <path d="M19 14v6M16 17h6" />
        </svg>
      );
    case "plus":
      return (
        <svg {...s}>
          <path d="M12 5.5v13M5.5 12h13" />
        </svg>
      );
    // The New row's plus, ringed the way the reference draws it. Separate from `plus`
    // because a bare plus is the right mark everywhere else it is used (a project row, a
    // menu entry) — those sit beside other icons at the same weight, and ringing them all
    // would make every one of them shout.
    case "dots":
      return (
        <svg {...s}>
          <circle cx="12" cy="5" r="1.4" />
          <circle cx="12" cy="12" r="1.4" />
          <circle cx="12" cy="19" r="1.4" />
        </svg>
      );
    case "speaker":
      return (
        <svg {...s}>
          <path d="M11 5.5 6.5 9H3.5v6h3L11 18.5z" />
          <path d="M15 9.5a4 4 0 0 1 0 5" />
          <path d="M17.8 7a7.5 7.5 0 0 1 0 10" />
        </svg>
      );
    case "check":
      return (
        <svg {...s}>
          <path d="M4.5 12.5l5 5 10-11" />
        </svg>
      );
    case "home":
      return (
        <svg {...s}>
          <path d="M4 11l8-6.5 8 6.5" />
          <path d="M6.5 9.8V19h11V9.8" />
        </svg>
      );
    case "plusCircle":
      return (
        <svg {...s}>
          <circle cx="12" cy="12" r="9" />
          <path d="M12 8.5v7M8.5 12h7" />
        </svg>
      );
    case "clock":
      return (
        <svg {...s}>
          <circle cx="12" cy="12" r="8.3" />
          <path d="M12 7.8V12l2.8 1.7" />
        </svg>
      );
    case "sliders":
      // Two rails, knob on each, rails broken around the knobs (owner pick 2026-07-11:
      // the three-rail version read as heavier than the mocks' two-rail glyph).
      return (
        <svg {...s}>
          <path d="M4 8h10M18 8h2M4 16h2M10 16h10" />
          <circle cx="16" cy="8" r="2" />
          <circle cx="8" cy="16" r="2" />
        </svg>
      );
    case "gear":
      return (
        <svg {...s}>
          <circle cx="12" cy="12" r="3" />
          <path d="M19 12a7 7 0 00-.1-1l2-1.6-2-3.4-2.3 1a7 7 0 00-1.7-1L16.5 3h-4l-.4 2.4a7 7 0 00-1.7 1l-2.3-1-2 3.4 2 1.6a7 7 0 000 2l-2 1.6 2 3.4 2.3-1a7 7 0 001.7 1l.4 2.4h4l.4-2.4a7 7 0 001.7-1l2.3 1 2-3.4-2-1.6c.1-.3.1-.7.1-1z" />
        </svg>
      );
  }
  return null;
}
