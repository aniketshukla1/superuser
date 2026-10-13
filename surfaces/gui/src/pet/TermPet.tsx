// Superuser's pet — a little CRT terminal that lives in the floating companion
// window (see Pet.tsx).
//
// It was once the mark with a face: both were a terminal window, so the mascot and
// the logo read as the same character. The mark has since been redrawn as a bare `#`
// (Icon.tsx case "logo") because the box dominated at small sizes. The pet kept the
// terminal, deliberately — it is only ever shown large, and the screen is what it
// emotes with; a `#` has nowhere to put eyes. So the two shapes diverge now, and the
// phosphor green below is the pet's own, not the brand's iris.
//
// Pure SVG; bob/blink/talk motion is CSS (styles.css). Fixed phosphor colours plus
// an outline and glow so it reads on any wallpaper — the window behind it is
// transparent.

export type MascotMood = "idle" | "happy" | "talking" | "thinking";

const CASE = "#11170f"; // the plastic shell
const SCREEN = "#050806"; // the glass
const GREEN = "#00e05a";
const DIM = "#0b3a1f";

export default function TermPet({ size = 132, mood = "idle" }: { size?: number; mood?: MascotMood }) {
  const lookUp = mood === "thinking";
  const eyeY = lookUp ? 56 : 62;
  const eyesClass = mood === "happy" ? undefined : "pet-eyes";

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 140 140"
      style={{ overflow: "visible", filter: "drop-shadow(0 10px 20px rgba(0,0,0,.5)) drop-shadow(0 0 14px rgba(0,224,90,.22))" }}
    >
      {/* stand */}
      <path d="M58 108 L82 108 L86 122 L54 122 Z" fill={CASE} stroke={GREEN} strokeWidth={2.5} strokeLinejoin="round" />
      <rect x="44" y="120" width="52" height="8" rx="4" fill={CASE} stroke={GREEN} strokeWidth={2.5} />

      {/* case */}
      <rect x="14" y="16" width="112" height="94" rx="14" fill={CASE} stroke={GREEN} strokeWidth={3} />
      {/* screen */}
      <rect x="24" y="26" width="92" height="66" rx="8" fill={SCREEN} />

      {/* face */}
      {mood === "happy" ? (
        // ^ ^
        <g fill="none" stroke={GREEN} strokeWidth={5} strokeLinecap="round">
          <path d="M46 62 L54 53 L62 62" />
          <path d="M78 62 L86 53 L94 62" />
        </g>
      ) : (
        <g className={eyesClass} fill={GREEN}>
          <rect x="46" y={eyeY - 7} width="15" height="14" rx="3" />
          <rect x="79" y={eyeY - 7} width="15" height="14" rx="3" />
        </g>
      )}

      {/* mouth — a prompt cursor */}
      {mood === "talking" ? (
        <rect className="pet-mouth-talk" x="62" y="74" width="16" height="9" rx="2" fill={GREEN} />
      ) : mood === "thinking" ? (
        <g fill={DIM}>
          <circle cx="62" cy="79" r="3" />
          <circle cx="70" cy="79" r="3" />
          <circle cx="78" cy="79" r="3" />
        </g>
      ) : mood === "happy" ? (
        <path d="M58 74 Q70 84 82 74" fill="none" stroke={GREEN} strokeWidth={4} strokeLinecap="round" />
      ) : (
        <rect x="60" y="76" width="20" height="4.5" rx="2" fill={GREEN} />
      )}

      {/* thinking: rising bits above the case */}
      {mood === "thinking" && (
        <g fill={GREEN} className="pet-think">
          <rect x="104" y="8" width="5" height="5" rx="1" opacity="0.55" />
          <rect x="114" y="-2" width="7" height="7" rx="1.5" />
        </g>
      )}

      {/* scanlines over the glass */}
      <g opacity="0.5">
        {[32, 38, 44, 50, 56, 62, 68, 74, 80, 86].map((y) => (
          <rect key={y} x="24" y={y} width="92" height="1" fill="#000" opacity="0.35" />
        ))}
      </g>

      {/* power LED */}
      <circle cx="118" cy="102" r="3" fill={GREEN} className="pet-led" />
    </svg>
  );
}
