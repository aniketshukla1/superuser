// Drawn rather than typed: a glyph inherits the text metrics and lands off-centre in
// a square button, which is exactly the kind of half-millimetre that makes a card
// look assembled instead of designed.
export function Chevron({ open }: { open: boolean }) {
  return (
    <svg viewBox="0 0 16 16" width="13" height="13" fill="none" aria-hidden="true">
      <path
        d={open ? "M4 6.5 8 10.5 12 6.5" : "M6.5 4 10.5 8 6.5 12"}
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function Cross() {
  return (
    <svg viewBox="0 0 16 16" width="13" height="13" fill="none" aria-hidden="true">
      <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}
