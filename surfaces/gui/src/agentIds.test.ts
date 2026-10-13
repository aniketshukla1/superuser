import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { readModule } from "./testing/source";

// Persona ids are DECLARED in Rust manifests and COMPARED in TypeScript, and a
// comparison against an id nothing can hold fails silently in the worst way: it is
// not dead code you can see, it reads as a working feature that simply never runs.
//
// That is what happened to `cowork`. It was the general assistant's id, the roster
// renamed it to `assistant`, and five conditions were left behind — the initial
// agent, a fallback in the surface guard, and three features gated on
// `agent === "cowork"`. The artifacts affordance in the top bar was among them, so
// nobody had seen it since the rename.
//
// Same shape as personaIcon.test.ts: one side declares, the other resolves, and
// nothing in between complains.

const BUILTINS = join(__dirname, "..", "..", "..", "src-tauri", "src", "core", "personas_builtin");

function shippedIds(): Set<string> {
  const ids = readdirSync(BUILTINS)
    .filter((f) => f.endsWith(".md"))
    .map((f) => {
      const src = readFileSync(join(BUILTINS, f), "utf8");
      return (src.match(/^id:\s*(.+)$/m)?.[1] ?? f.replace(/\.md$/, "")).trim();
    });
  return new Set(ids);
}

/** Every `agent === "x"` / `agent !== "x"` comparison in a source file.
 *
 *  Comments are stripped first. They are where this file's own examples live, and a
 *  reader that counts prose finds the bug it was written to describe. */
function agentComparisons(src: string): string[] {
  const code = src.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/[^\n]*/g, "");
  const out: string[] = [];
  for (const m of code.matchAll(/\bagent\s*[=!]==\s*"([^"]+)"/g)) out.push(m[1]);
  return [...new Set(out)];
}

describe("persona ids the client compares against", () => {
  it("reads comparisons out of source, and not out of prose about them", () => {
    // Proven against a fixture rather than against App.tsx, because "App.tsx has
    // none left" is a legitimate — and better — answer, and a reader whose only
    // health check is "it found something" starts passing vacuously the moment the
    // last one is deleted. Which is what happened.
    const sample = `
      // agent === "ghost-in-a-comment"
      if (agent === "coder" && x) {}
      const y = agent !== "assistant";
      /* agent === "ghost-in-a-block" */
    `;
    expect(agentComparisons(sample).sort()).toEqual(["assistant", "coder"]);
  });

  it("App.tsx only tests the agent against ids the roster actually ships", () => {
    const src = readModule(join(__dirname, "App.tsx"));
    const ids = shippedIds();
    const compared = agentComparisons(src);

    const unknown = compared.filter((id) => !ids.has(id));
    expect(
      unknown,
      `these are compared against but no shipped persona has them, so the branch can never run:\n  ${unknown.join(
        "\n  ",
      )}\n\nA renamed persona leaves its old id in conditions that then read as working features.`,
    ).toEqual([]);
  });

  it("the general assistant this client falls back to is one that exists", () => {
    // Rust calls it DEFAULT_PERSONA_ID and every fallback here has to agree with it,
    // or a first run — and the surface guard — lands on nothing.
    const rust = readModule(join(__dirname, "..", "..", "..", "src-tauri", "src", "core", "personas.rs"));
    const declared = rust.match(/DEFAULT_PERSONA_ID:\s*&str\s*=\s*"([^"]+)"/)?.[1];
    expect(declared, "DEFAULT_PERSONA_ID moved").toBeTruthy();
    expect(shippedIds().has(declared!)).toBe(true);

    const app = readModule(join(__dirname, "App.tsx"));
    const client = app.match(/const GENERAL_ASSISTANT\s*=\s*"([^"]+)"/)?.[1];
    expect(client, "App.tsx no longer names a general assistant").toBeTruthy();
    expect(client).toBe(declared);
  });
});
