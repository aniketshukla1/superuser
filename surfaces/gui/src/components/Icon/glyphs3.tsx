import type { ReactElement } from "react";
import type { IconName } from "./IconName";
import type { SvgProps } from "./svgProps";

/** Glyphs "inbox" … "trash". */
export function glyphs3(name: IconName, s: SvgProps): ReactElement | null {
  switch (name) {
    case "inbox":
      return (
        <svg {...s}>
          <path d="M22 12h-6l-2 3h-4l-2-3H2" />
          <path d="M5.4 5.1 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.4-6.9A2 2 0 0 0 16.8 4H7.2a2 2 0 0 0-1.8 1.1z" />
        </svg>
      );
    case "code":
      return (
        <svg {...s}>
          <path d="M8.5 8.5 4.5 12l4 3.5M15.5 8.5l4 3.5-4 3.5" />
        </svg>
      );
    case "chat":
      return (
        <svg {...s}>
          <path d="M5 5.5h14c.8 0 1.5.7 1.5 1.5v7c0 .8-.7 1.5-1.5 1.5H9.5L5.5 19v-3H5c-.8 0-1.5-.7-1.5-1.5V7c0-.8.7-1.5 1.5-1.5z" />
        </svg>
      );
    case "wrench":
      return (
        <svg {...s}>
          <path d="M14.6 6.4a3.8 3.8 0 0 0-5 4.9l-6 6a1.6 1.6 0 0 0 2.3 2.3l6-6a3.8 3.8 0 0 0 4.9-5l-2.4 2.4-2-.5-.5-2 2.7-2.1z" />
        </svg>
      );
    case "search":
      return (
        <svg {...s}>
          <circle cx="10.5" cy="10.5" r="6.3" />
          <path d="M15.2 15.2 20 20" />
        </svg>
      );
    case "diamond":
      return (
        <svg {...s} fill="currentColor" stroke="none">
          <path d="M12 3.2 20.8 12 12 20.8 3.2 12z" />
        </svg>
      );
    case "pencil":
      return (
        <svg {...s}>
          <path d="M4 20l1.1-3.9L15.6 5.6a1.6 1.6 0 0 1 2.3 0l.5.5a1.6 1.6 0 0 1 0 2.3L7.9 18.9 4 20z" />
          <path d="M14.5 6.7l2.8 2.8" />
        </svg>
      );
    case "branch":
      return (
        <svg {...s}>
          <circle cx="7" cy="6" r="2.1" />
          <circle cx="7" cy="18" r="2.1" />
          <circle cx="17" cy="8" r="2.1" />
          <path d="M7 8.1v7.8M7 12.5c5.4 0 10-.4 10-2.4" />
        </svg>
      );
    case "arrowRight":
      // The mirror of arrowLeft, drawn rather than transformed: a CSS flip would
      // reverse the stroke's round caps too, and at 15px that reads as a different
      // weight beside its twin.
      return (
        <svg {...s}>
          <path d="M5 12h14M13 6l6 6-6 6" />
        </svg>
      );
    case "arrowLeft":
      return (
        <svg {...s}>
          <path d="M19 12H5M11 6l-6 6 6 6" />
        </svg>
      );
    case "copy":
      return (
        <svg {...s}>
          <rect x="8" y="8" width="11" height="11" rx="2" />
          <path d="M5 15.5H4.8A1.8 1.8 0 0 1 3 13.7V4.8A1.8 1.8 0 0 1 4.8 3h8.9a1.8 1.8 0 0 1 1.8 1.8V5" />
        </svg>
      );
    case "refresh":
      return (
        <svg {...s}>
          <path d="M20 11a8 8 0 0 0-14.3-4.9L4 8M4 4v4h4M4 13a8 8 0 0 0 14.3 4.9L20 16M16 16h4v4" />
        </svg>
      );
    case "panelClose":
      return (
        <svg {...s}>
          <rect x="4" y="5" width="16" height="14" rx="2" />
          <path d="M14.5 5v14M8 10l3 2-3 2" />
        </svg>
      );
    case "panelOpen":
      return (
        <svg {...s}>
          <rect x="4" y="5" width="16" height="14" rx="2" />
          <path d="M9.5 5v14M16 10l-3 2 3 2" />
        </svg>
      );
    case "plug":
      return (
        <svg {...s}>
          <path d="M9 7V3M15 7V3M7 7h10v4a5 5 0 0 1-10 0V7zM12 16v5" />
        </svg>
      );
    case "audit":
      return (
        <svg {...s}>
          <path d="M7 4h10a2 2 0 0 1 2 2v14H7a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z" />
          <path d="M8.5 9h7M8.5 13h7M8.5 17H13" />
        </svg>
      );
    case "chevronRight":
      return (
        <svg {...s}>
          <path d="m9 6 6 6-6 6" />
        </svg>
      );
    case "chevronDown":
      return (
        <svg {...s}>
          <path d="m6 9 6 6 6-6" />
        </svg>
      );
    case "moreHorizontal":
      return (
        <svg {...s}>
          <circle cx="6.5" cy="12" r="1.2" fill="currentColor" stroke="none" />
          <circle cx="12" cy="12" r="1.2" fill="currentColor" stroke="none" />
          <circle cx="17.5" cy="12" r="1.2" fill="currentColor" stroke="none" />
        </svg>
      );
    case "pin":
      return (
        <svg {...s}>
          <path d="M14.2 4.8 19.2 9.8M9.2 14.4 4.7 18.9M8 8.6l7.4 7.4M8.4 8.2l2.3-3.4 8.5 8.5-3.4 2.3c-.7.5-1.6.4-2.2-.2L8.6 10.4c-.6-.6-.7-1.5-.2-2.2z" />
        </svg>
      );
    case "archive":
      return (
        <svg {...s}>
          <path d="M4.5 7.5h15M6 7.5v10.2c0 1 .8 1.8 1.8 1.8h8.4c1 0 1.8-.8 1.8-1.8V7.5M5.6 4.5h12.8c.6 0 1.1.5 1.1 1.1v1.9h-15V5.6c0-.6.5-1.1 1.1-1.1z" />
          <path d="M9.5 11.5h5" />
        </svg>
      );
    case "trash":
      return (
        <svg {...s}>
          <path d="M4.5 7h15" />
          <path d="M10 11v6M14 11v6" />
          <path d="M6.5 7l.9 12c.1.9.8 1.5 1.7 1.5h7.8c.9 0 1.6-.6 1.7-1.5l.9-12" />
          <path d="M9.2 7V4.9c0-.5.4-.9.9-.9h3.8c.5 0 .9.4.9.9V7" />
        </svg>
      );
  }
  return null;
}
