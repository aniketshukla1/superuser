import type { CSSProperties } from "react";
import { GLYPHS, rng } from "./script";

const NAME = "superuser";
const TAGLINE = ["Sees everything", "Asks before it acts", "Stays on this machine"];

/**
 * The name, rolling in letter by letter — each letter scrolls up through a few glyphs and
 * lands, left to right (the owner's "scrolling"), then the word glows on the matte black.
 *
 * Each cell is as wide as its real letter (a hidden copy sets it), so the word holds its
 * shape while the glyphs run through it. The real word is in an off-screen copy too, for
 * a screen reader and for anything that reads the text. Not a heading: the page's outline
 * belongs to the app underneath, which already has its h1.
 */
export function Name({ at, each, glow, still }: { at: number; each: number; glow: number; still?: boolean }) {
  if (still)
    return (
      <div className="film-name is-still">
        <span className="boot-head">{NAME}</span>
      </div>
    );
  return (
    <div className="film-name" style={{ "--glow": `${glow}ms` } as CSSProperties}>
      <span className="film-sr boot-head">{NAME}</span>
      {NAME.split("").map((ch, i) => {
        const r = rng(i * 31 + 7);
        const run = Array.from({ length: 5 }, () => GLYPHS[Math.floor(r() * GLYPHS.length)]);
        return (
          <span key={i} className="film-roll" aria-hidden="true">
            <span className="film-roll-size">{ch}</span>
            <span className="film-roll-run" style={{ animationDelay: `${at + i * each}ms` }}>
              {run.map((g, k) => (
                <span key={k}>{g}</span>
              ))}
              <span>{ch}</span>
            </span>
          </span>
        );
      })}
    </div>
  );
}

/** The three promises, word by word. */
export function Tagline({ at, still }: { at: number; still?: boolean }) {
  let n = 0;
  return (
    <p className={"film-tagline" + (still ? " is-still" : "")}>
      {TAGLINE.map((phrase, p) => (
        <span key={p}>
          {phrase.split(" ").map((w) => (
            <span key={w + n} className="film-word" style={{ animationDelay: `${at + n++ * 55}ms` }}>
              {w}
            </span>
          ))}
          {p < TAGLINE.length - 1 && (
            <span className="film-word film-dot" style={{ animationDelay: `${at + n * 55}ms` }}>
              ·
            </span>
          )}
        </span>
      ))}
    </p>
  );
}
