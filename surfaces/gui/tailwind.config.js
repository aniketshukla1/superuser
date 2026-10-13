/** Tailwind config — mirrors platform/ui-mocks/redesign.html so the app can use the mock's
 *  exact utility classes. Colors map to the CSS custom properties already defined in styles.css
 *  (so light/dark theming flows through one source of truth). */
// Tokens are hex CSS vars (shared with styles.css), which Tailwind can't alpha-multiply for
// `/NN` opacity utilities. Wrap each in color-mix so `bg-panel/70` etc. work, while bare
// `var(--x)` usage in styles.css stays intact. (color-mix is supported in the Chromium webview.)
const tok = (name) => ({ opacityValue }) =>
  opacityValue === undefined
    ? `var(${name})`
    : `color-mix(in srgb, var(${name}) calc(${opacityValue} * 100%), transparent)`;

/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["selector", '[data-theme="dark"]'],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        paper: tok("--paper"),
        panel: tok("--panel"),
        ink: tok("--ink"),
        muted: tok("--muted"),
        faint: tok("--faint"),
        line: tok("--line"),
        lineStrong: tok("--line-strong"),
        accent: tok("--accent"),
        accentSoft: tok("--accent-soft"),
        onAccent: tok("--on-accent"),
        ok: tok("--ok"),
        okSoft: tok("--ok-soft"),
        okLine: tok("--ok-line"),
        warnInk: tok("--warn-ink"),
        warnSoft: tok("--warn-soft"),
        danger: tok("--danger"),
        dangerSoft: tok("--danger-soft"),
        tealInk: tok("--teal-ink"),
        tealSoft: tok("--teal-soft"),
        tealLine: tok("--teal-line"),
        solid: tok("--solid"),
        onSolid: tok("--on-solid"),
      },
      // Point at the same variables styles.css defines, the way the colours already
      // do. These used to be hardcoded copies of the old stacks, so changing
      // `--sans` to IBM Plex Sans moved every plain element and left every
      // `font-sans` utility on the previous font — one app, two typefaces, and no
      // error anywhere. A second copy of a value is a second thing to forget.
      fontFamily: {
        sans: "var(--sans)",
        mono: "var(--mono)",
      },
// EVERY radius key points at the scale in styles.css — including Tailwind's
      // own `sm/md/lg/xl/2xl`, which is where the drift came from: `rounded-lg` is
      // used 106 times and was Tailwind's default 8px, a number nobody here chose.
      // `rounded-xl` (24 uses) was 12px, off the scale entirely.
      //
      // The old names are kept as aliases rather than renamed across 200 call sites:
      // a rename that big is a diff nobody can review, and the point is to make the
      // VALUES right, not to relitigate what each one is called.
      borderRadius: {
        none: "0",
        xs: "var(--r-xs)",
        sm: "var(--r-sm)",
        md: "var(--r-sm)",
        DEFAULT: "var(--r-sm)",
        lg: "var(--r-md)",
        xl: "var(--r-lg)",
        "2xl": "var(--r-lg)",
        "3xl": "var(--r-lg)",
        full: "var(--r-pill)",
        // Named for what they are, for new code.
        row: "var(--r-sm)",
        card: "var(--r-xs)",
        xl2: "var(--r-lg)",
      },
      // Type: size and line-height as one decision, so `text-md` cannot pick up a
      // line-height from whatever it inherits. `text-[12.5px]` sets ONLY the size —
      // that is not a Tailwind quirk to work around, it is the reason ~400 call
      // sites have emergent row heights.
      // Prose gets its own line-height token. `leading-relaxed` is a UNITLESS 1.625,
      // so it computes to 24.375px on a 15px font — a fractional line box that
      // rounds per element, which is the same failure the --t-* pairs exist to end.
      // `--t-prose-lh` is the whole-pixel answer the stylesheet already used for
      // chat bubbles and nothing else could reach.
      // Override Tailwind's own `tracking-tight` (-0.025em) rather than adding a
      // name beside it: the thirteen call sites already say `tracking-tight` and
      // mean "the heading tightening", not "-0.025em specifically".
      letterSpacing: {
        tight: "var(--ls-tight)",
      },
      lineHeight: {
        prose: "var(--t-prose-lh)",
      },
      fontSize: {
        xs: ["var(--t-xs)", { lineHeight: "var(--t-xs-lh)" }],
        sm: ["var(--t-sm)", { lineHeight: "var(--t-sm-lh)" }],
        base: ["var(--t-md)", { lineHeight: "var(--t-md-lh)" }],
        md: ["var(--t-md)", { lineHeight: "var(--t-md-lh)" }],
        lg: ["var(--t-lg)", { lineHeight: "var(--t-lg-lh)" }],
        xl: ["var(--t-xl)", { lineHeight: "var(--t-xl-lh)" }],
      },
    },
  },
  plugins: [],
};
