import { readFileSync } from "node:fs";
import path from "node:path";

export interface FromSource {
  personas: { id: string; name: string; icon: string; tagline: string; family: string; tools: string[]; mode: string }[];
  presets: { id: string; name: string; when: string; cost: string }[];
  fields: { field: string; means: string }[];
}
export const fromSource: FromSource = JSON.parse(
  readFileSync(path.join(process.cwd(), "e2e-demo", "from-source.json"), "utf8"),
);

export const NOW = Math.floor(Date.now() / 1000);
export const ago = (minutes: number) => NOW - minutes * 60;
export const stamp = (minutes: number) => new Date((NOW - minutes * 60) * 1000).toISOString().slice(0, 19).replace("T", " ");

export const MODEL = "gemma4:12b";
export const MODEL_LABELS = {
  "gemma4:12b": "gemma4:12b · Ollama · 131k context",
  "llama3.2:latest": "llama3.2:latest · Ollama · 131k context",
  "anthropic:claude-opus-5": "Claude Opus 5 · Anthropic",
};
