/** The attributes every glyph's <svg> shares. */
export function svgProps(size: number, className?: string) {
  return {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    // 1.5, not 1.7: most of this set renders at 13–16px next to 12.5px text, and
    // 1.7 was heavier than the type it sat beside — the icons read as the loudest
    // thing in a row they are meant to label.
    strokeWidth: 1.5,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    className,
    "aria-hidden": true,
  };
}

export type SvgProps = ReturnType<typeof svgProps>;
