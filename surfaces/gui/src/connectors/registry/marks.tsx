import type { SimpleIcon } from "simple-icons";

// The building blocks of the logo registry, and the glyphs drawn for this app.

// `JSX` is global with the react-jsx runtime and @types/react.
export type LogoComponent = () => JSX.Element;

export interface ConnectorRegistryEntry {
  label: string;
  logo: LogoComponent;
}

/** A filled 24×24 single-path mark in `currentColor`. */
export function pathLogo(d: string): LogoComponent {
  return function PathMark() {
    return (
      <svg viewBox="0 0 24 24" width="100%" height="100%" fill="currentColor" aria-hidden="true">
        <path d={d} />
      </svg>
    );
  };
}

/** A registry entry straight from the icon package. */
export const brand = (icon: SimpleIcon): ConnectorRegistryEntry => ({ label: icon.title, logo: pathLogo(icon.path) });

/** A line glyph: 24×24, a 1.8 stroke in `currentColor` with round ends and corners. */
function lineGlyph(shapes: JSX.Element): LogoComponent {
  return function LineMark() {
    return (
      <svg
        viewBox="0 0 24 24"
        width="100%"
        height="100%"
        fill="none"
        stroke="currentColor"
        strokeWidth={1.8}
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        {shapes}
      </svg>
    );
  };
}

// Utilities.
export const EmailLogo = lineGlyph(
  <>
    <rect x="3.5" y="5.5" width="17" height="13" rx="2" />
    <path d="m4 7 8 6 8-6" />
  </>,
);
export const BrowserLogo = lineGlyph(
  <>
    <circle cx="12" cy="12" r="8.5" />
    <ellipse cx="12" cy="12" rx="3.6" ry="8.5" />
    <path d="M3.5 12h17" />
  </>,
);
export const McpLogo = lineGlyph(
  <>
    <circle cx="12" cy="12.5" r="2.1" />
    <circle cx="5.5" cy="5.5" r="1.7" />
    <circle cx="18.5" cy="5.5" r="1.7" />
    <circle cx="12" cy="20" r="1.7" />
    <path d="m6.8 6.8 3.7 4.1M17.2 6.8l-3.7 4.1M12 14.6v3.7" />
  </>,
);
export const PlugLogo = lineGlyph(<path d="M9 3.5v3.5M15 3.5v3.5M7 7h10v3.5a5 5 0 0 1-10 0zM12 15.5V21" />);

// Brands without a published one-colour mark, drawn simply and tinted with their real colour.
export const AttioLogo = lineGlyph(<path d="M5 20 12 4l7 16M8.3 13.8h7.4" />);
// monday.com's mark is three offset bars.
export const MondayLogo = lineGlyph(<path d="M4 8h10M4 12h16M4 16h7" />);
export const AmplitudeLogo = lineGlyph(<path d="M3 13h3.5L9 6l5 12 2.5-7H21" />);
export const ApolloLogo = lineGlyph(
  <>
    <circle cx="9.5" cy="14.5" r="5.5" />
    <path d="m14 10 6-6M16 4h4v4" />
  </>,
);
export const DescriptLogo = lineGlyph(<path d="M4 6h15M4 10.5h10M4 15h13M4 19.5h7" />);
export const ClayLogo = lineGlyph(<path d="M4 18.5a8 8 0 0 1 16 0zM2 18.5h20" />);
export const CloseLogo = lineGlyph(
  <>
    {[0, 60, 120].map((turn) => (
      <ellipse key={turn} cx="12" cy="12" rx="8.5" ry="3.5" transform={`rotate(${turn} 12 12)`} />
    ))}
  </>,
);
export const DocusignLogo = lineGlyph(<path d="M15 4.5 19.5 9 9 19.5H4.5V15zM13 6.5l4.5 4.5M4 21h16" />);
export const HunterLogo = lineGlyph(
  <>
    <circle cx="12" cy="12" r="6.5" />
    <path d="M12 3v3.5M12 17.5V21M3 12h3.5M17.5 12H21" />
    <circle cx="12" cy="12" r="1.5" fill="currentColor" stroke="none" />
  </>,
);
