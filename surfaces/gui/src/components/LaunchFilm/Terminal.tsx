import type { TermLine } from "./work";

/**
 * A terminal running far faster than anyone could read, down one edge of the screen —
 * the code running behind the work. The lines are written twice and the column scrolls
 * by exactly one copy, so it runs without a seam for as long as the film is up.
 */
export function Terminal({ lines, side, seconds }: { lines: TermLine[]; side: "left" | "right"; seconds: number }) {
  return (
    <div className={`film-terminal is-${side}`} aria-hidden="true">
      <div className="film-scroll" style={{ animationDuration: `${seconds}s` }}>
        {[...lines, ...lines].map((line, i) => (
          <div key={i} className={`film-tone-${line.tone}`}>
            {line.text}
          </div>
        ))}
      </div>
    </div>
  );
}

let grain: string | undefined;

/** Film grain, made once: a square of noise painted to a canvas, tiled and jittered by CSS. */
export function grainUrl(): string {
  if (grain !== undefined) return grain;
  try {
    const size = 160;
    const canvas = document.createElement("canvas");
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext("2d");
    if (!ctx) return (grain = "");
    const image = ctx.createImageData(size, size);
    for (let i = 0; i < image.data.length; i += 4) {
      const v = Math.random() * 255;
      image.data[i] = v;
      image.data[i + 1] = v;
      image.data[i + 2] = v;
      image.data[i + 3] = Math.random() * 40;
    }
    ctx.putImageData(image, 0, 0);
    grain = canvas.toDataURL("image/png");
  } catch {
    grain = "";
  }
  return grain;
}

const COMMAND = "do --anything";

/** The first thing in the dark: a command, typed. */
export function Command({ at }: { at: number }) {
  return (
    <p className="film-command" aria-hidden="true">
      <span className="film-prompt-dim">superuser ~ </span>
      {COMMAND.split("").map((ch, i) => (
        <span key={i} className="film-char" style={{ animationDelay: `${at + 160 + i * 42}ms` }}>
          {ch}
        </span>
      ))}
      <span className="film-cursor" />
    </p>
  );
}
