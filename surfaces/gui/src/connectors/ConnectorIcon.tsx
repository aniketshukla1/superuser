// A connector's logo, looked up in the registry by its `logo` id and painted in its
// `brand_color`. The colour arrives as a prop from the API data, so there is no table of
// colours in CSS; an unknown or empty id shows the neutral plug, still in the given colour.
// `--brand`, and `--brand-soft` for the badge's tint, are set inline from that colour.

import type { CSSProperties } from "react";
import { resolveConnector } from "./registry";

/** The grey used when a connector has no colour; the descriptor uses the same default. */
export const NEUTRAL = "#6b7280";

/** Anything that carries a connector's look; a full `Connector` fits. */
export interface ConnectorVisual {
  logo?: string;
  brand_color?: string;
  title?: string;
  label?: string;
}

/** The red, green and blue of a `#rgb` or `#rrggbb` colour, or null when it isn't one. */
function channels(hex: string): [number, number, number] | null {
  let digits = hex.trim().replace(/^#/, "");
  if (digits.length === 3) digits = [...digits].map((d) => d + d).join("");
  if (!/^[0-9a-fA-F]{6}$/.test(digits)) return null;
  return [0, 2, 4].map((at) => parseInt(digits.slice(at, at + 2), 16)) as [number, number, number];
}

/** A hex colour as `rgba()` at `alpha`; "" when the hex is malformed. */
export function hexToRgba(hex: string, alpha: number): string {
  const rgb = channels(hex);
  return rgb ? `rgba(${rgb.join(", ")}, ${alpha})` : "";
}

/** Marks that are nearly black (GitHub's #1f2328, Notion, …) disappear on a dark panel, so
 *  they are flagged for dark-mode CSS: a bare icon's mark is mixed toward the theme's ink,
 *  while a badge sits on a light plate (styles.css), where the original mark is right. */
export function isDarkMark(color: string): boolean {
  const rgb = channels(color);
  if (!rgb) return false;
  const [r, g, b] = rgb.map((v) => v / 255);
  // Relative luminance (Rec. 709 weights): under a quarter reads as near-black.
  return 0.2126 * r + 0.7152 * g + 0.0722 * b < 0.25;
}

const colourOf = (connector: ConnectorVisual) => (connector.brand_color ?? "").trim() || NEUTRAL;

/** `--brand` (the mark's colour) and `--brand-soft` (a faint tint behind it). */
function brandStyle(color: string, size: number): CSSProperties {
  const tint = hexToRgba(color, 0.12) || "var(--line)";
  return { ["--brand"]: color, ["--brand-soft"]: tint, color, width: size, height: size } as CSSProperties;
}

const nameOf = (connector: ConnectorVisual, registryLabel: string, title?: string) =>
  title ?? connector.label ?? connector.title ?? registryLabel;

type Props = { connector: ConnectorVisual; size?: number; title?: string };

/** The bare mark, in the brand's colour. */
export function ConnectorIcon({ connector, size = 18, title }: Props) {
  const color = colourOf(connector);
  const { key, entry } = resolveConnector(connector.logo);
  const label = nameOf(connector, entry.label, title);
  const Mark = entry.logo;
  return (
    <span
      className="connector-icon"
      data-logo={key}
      data-brand={key}
      data-dark-mark={isDarkMark(color) || undefined}
      role="img"
      aria-label={label}
      title={label}
      style={brandStyle(color, size)}
    >
      <Mark />
    </span>
  );
}

/** A rounded square: the brand's soft tint behind the mark in the brand's colour. */
export function ConnectorBadge({ connector, size = 32, title }: Props) {
  const { key, entry } = resolveConnector(connector.logo);
  const label = nameOf(connector, entry.label, title);
  return (
    <span
      className="connector-badge"
      data-logo={key}
      data-brand={key}
      role="img"
      aria-label={label}
      title={label}
      style={brandStyle(colourOf(connector), size)}
    >
      <ConnectorIcon connector={connector} size={Math.round(size * 0.56)} title={label} />
    </span>
  );
}
