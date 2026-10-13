import type { ReactElement } from "react";
import type { IconName } from "./IconName";
import type { SvgProps } from "./svgProps";

/** Glyphs "book" … "coins". */
export function glyphs1(name: IconName, s: SvgProps): ReactElement | null {
  switch (name) {
    case "book":
      // A playbook — Skills are the worker's recipe book (Settings ▸ Skills).
      // Hardcover with a full spine + two text lines: "written instructions inside".
      // (Owner-picked from a 15px-preview comparison, 2026-07-27.)
      return (
        <svg {...s}>
          <path d="M6 2h12a2 2 0 0 1 2 2v16a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z" />
          <path d="M8.5 2v20" />
          <path d="M11.5 8.5h5" />
          <path d="M11.5 12h3.5" />
        </svg>
      );
    case "globe":
      // The in-app browser. Meridian + equator rather than a full graticule — at 13px
      // any more lines collapse into a grey disc.
      return (
        <svg {...s}>
          <circle cx="12" cy="12" r="9" />
          <path d="M3 12h18" />
          <path d="M12 3a14 14 0 0 1 0 18a14 14 0 0 1 0-18z" />
        </svg>
      );
    case "sparkle":
      // Filled 4-point twinkle — crisp at small sizes.
      return (
        <svg {...s} fill="currentColor" stroke="none">
          <path d="M12 2.4c.5 4.7 2.5 6.7 7.2 7.2-4.7.5-6.7 2.5-7.2 7.2-.5-4.7-2.5-6.7-7.2-7.2 4.7-.5 6.7-2.5 7.2-7.2z" />
        </svg>
      );
    case "logo":
      // The Superuser mark — root's shell prompt, drawn as itself. In Unix `$` is a
      // normal user and `#` is root, so the prompt IS the name.
      //
      // This used to wrap the idea in a terminal window with a chevron inside it,
      // which cost the mark its legibility: at 13px the box dominated and the part
      // carrying the meaning was three pixels of chevron. A bare `#` reads at 11px,
      // needs no colour to be recognised, and is the same shape at every size. The
      // block caret beside it belongs to the splash, where there is room for it.
      //
      // Slanted verticals because that is how a `#` is actually cut — upright bars
      // read as a grid or a table icon instead.
      return (
        <svg {...s}>
          <path d="M9.4 4.2 L7.2 19.8" />
          <path d="M16.8 4.2 L14.6 19.8" />
          <path d="M4.6 9.6 H19.4" />
          <path d="M3.8 14.4 H18.6" />
        </svg>
      );
    case "sidebar":
      // Clean "sidebar panel" toggle — rounded rect + divider, no chevron (one glyph both states).
      return (
        <svg {...s}>
          <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
          <path d="M9 4.5v15" />
        </svg>
      );
    case "sidebarRight":
      // Right-panel counterpart of "sidebar" — same rounded rect + divider, mirrored.
      return (
        <svg {...s}>
          <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
          <path d="M15 4.5v15" />
        </svg>
      );
    case "signOut":
      return (
        <svg {...s}>
          <path d="M9 4H5.5A1.5 1.5 0 0 0 4 5.5v13A1.5 1.5 0 0 0 5.5 20H9" />
          <path d="M15 8l4 4-4 4M19 12H9" />
        </svg>
      );
    case "folder":
      // Clean single-tab folder (Lucide-style) — no internal divider line.
      return (
        <svg {...s}>
          <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9l-.81-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
        </svg>
      );
    case "shield":
      return (
        <svg {...s}>
          <path d="M12 3.2l7 2.8v5.1c0 4.3-2.9 7.4-7 9.7-4.1-2.3-7-5.4-7-9.7V6l7-2.8z" />
          <path d="M9.3 12.1l1.9 1.9 3.5-3.6" />
        </svg>
      );
    case "file":
      // Document with a folded corner + text lines.
      return (
        <svg {...s}>
          <path d="M6.5 3.5h7L18 8v11a1.5 1.5 0 0 1-1.5 1.5h-10A1.5 1.5 0 0 1 5 19V5a1.5 1.5 0 0 1 1.5-1.5z" />
          <path d="M13.5 3.5V8H18M8.5 12.5h7M8.5 16h5" />
        </svg>
      );
    case "fileCode":
      // Document with </> marks (html/code).
      return (
        <svg {...s}>
          <path d="M6.5 3.5h7L18 8v11a1.5 1.5 0 0 1-1.5 1.5h-10A1.5 1.5 0 0 1 5 19V5a1.5 1.5 0 0 1 1.5-1.5z" />
          <path d="M13.5 3.5V8H18M10.2 12l-2 2.2 2 2.2M13.3 12l2 2.2-2 2.2" />
        </svg>
      );
    case "image":
      // Picture frame: sun + mountains.
      return (
        <svg {...s}>
          <rect x="4" y="5" width="16" height="14" rx="2" />
          <circle cx="9" cy="10" r="1.6" />
          <path d="M4.5 16.5l4.5-4 3.5 3 3-2.5 4 3.5" />
        </svg>
      );
    case "table":
      // Spreadsheet grid (csv/xlsx).
      return (
        <svg {...s}>
          <rect x="4" y="5" width="16" height="14" rx="2" />
          <path d="M4 10h16M10 10v9M4 14.5h16" />
        </svg>
      );
    case "paperclip":
      // The attach mark the composer's menu is named after ("Add files or photos").
      return (
        <svg {...s} className={s.className} aria-hidden="true">
          <path d="M21.4 11.05 12.25 20.2a5.5 5.5 0 0 1-7.78-7.78l9.2-9.19a3.67 3.67 0 1 1 5.18 5.18l-9.2 9.2a1.83 1.83 0 1 1-2.59-2.6l8.5-8.49" />
        </svg>
      );
    case "helpCircle":
      return (
        <svg {...s} className={s.className} aria-hidden="true">
          <circle cx="12" cy="12" r="9" />
          <path d="M9.6 9.3a2.5 2.5 0 0 1 4.86.83c0 1.67-2.5 2.5-2.5 2.5" />
          <path d="M12 17h.01" />
        </svg>
      );
    case "users":
      // Two people, the second half-behind — a team, not a crowd. Drawn at the same
      // 1.5 stroke as the rest so it does not read louder in a sidebar row.
      return (
        <svg {...s}>
          <circle cx="9.5" cy="8" r="3.25" />
          <path d="M3.5 19.5a6 6 0 0 1 12 0" />
          <path d="M16 5.2a3.25 3.25 0 0 1 0 5.6" />
          <path d="M17.2 14.4a6 6 0 0 1 3.3 5.1" />
        </svg>
      );
    case "coins":
      // A stack of three, seen edge-on. A single coin reads as a clock face at 15px;
      // the stack is what makes it money.
      return (
        <svg {...s}>
          <ellipse cx="12" cy="6.5" rx="7" ry="3" />
          <path d="M5 6.5v5c0 1.66 3.13 3 7 3s7-1.34 7-3v-5" />
          <path d="M5 11.5v5c0 1.66 3.13 3 7 3s7-1.34 7-3v-5" />
        </svg>
      );
  }
  return null;
}
