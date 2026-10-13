// The mark: root's shell prompt plus the caret the machine waits at. `#` is root
// where everyone else gets `$`, so the glyph is the product's name rather than a
// picture standing in for it.
//
// Text, not an SVG: it inherits the type scale, it selects, and a screen reader
// reading "superuser" beside it does not also announce a decorative image. The
// caret is the only part that is purely decoration, so that is the part hidden.
export function Mark({ size = 15 }: { size?: number }) {
  return (
    <span className="mark" style={{ fontSize: size }} aria-hidden="true">
      <span className="hash">#</span>
      <span className="caret" />
    </span>
  );
}
