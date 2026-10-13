import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { personaGlyph } from "./personaIcon";

// A persona's icon is DECLARED in Rust (a manifest's `icon:`) and RESOLVED here,
// against a curated set the manifest cannot see. So a manifest can name a glyph
// that does not exist and nothing fails — `personaGlyph` quietly returns the family
// fallback, and the assistant wears the generic mark.
//
// It had already happened twice: `lead` asked for `users`, which is in no icon set
// at all, and `assistant` asked for `sparkles` where the name is `sparkle` — that
// one looked right purely because the fallback is also sparkle.
//
// Same shape as the API contract: declared in one language, resolved in another,
// with silence in between.

const BUILTINS = join(__dirname, "..", "..", "..", "..", "src-tauri", "src", "core", "personas_builtin");

function manifests(): { file: string; icon: string; family: string }[] {
  return readdirSync(BUILTINS)
    .filter((f) => f.endsWith(".md"))
    .map((f) => {
      const src = readFileSync(join(BUILTINS, f), "utf8");
      return {
        file: f,
        icon: (src.match(/^icon:\s*(.+)$/m)?.[1] ?? "").trim(),
        family: (src.match(/^family:\s*(.+)$/m)?.[1] ?? "").trim(),
      };
    });
}

describe("persona icons", () => {
  it("every shipped manifest names a glyph the client can resolve", () => {
    const offenders = manifests()
      .filter((m) => m.icon && personaGlyph(m.icon, m.family) !== m.icon)
      .map((m) => `${m.file}: icon "${m.icon}" resolves to "${personaGlyph(m.icon, m.family)}"`);
    expect(
      offenders,
      `These fall through to the family fallback and wear the wrong mark:\n${offenders.join("\n")}`,
    ).toEqual([]);
  });

  it("still falls back rather than throwing on an unknown name", () => {
    // The fallback is right; it is being reached silently that is wrong.
    expect(personaGlyph("no-such-glyph", "code")).toBe("code");
    expect(personaGlyph(undefined, undefined)).toBe("sparkle");
  });
});
