import { describe, expect, it } from "vitest";
import { chipName, menuName, stripProvider } from "./modelName";

const PROVIDERS = ["ollama", "anthropic", "openai", "gemini", "custom"];

describe("shortening a model id", () => {
  it("drops a provider prefix", () => {
    expect(chipName("anthropic:claude-opus-4-8", undefined, PROVIDERS)).toBe("claude-opus-4-8");
  });

  // The bug this module exists to prevent: an Ollama tag is not a provider. Splitting
  // on the colon and keeping the TAIL turns "gemma4:12b" into "12b" — a model nobody
  // has heard of. The tail is what goes; the head is the name.
  it("does NOT mistake an Ollama tag for a provider", () => {
    expect(stripProvider("gemma4:12b", PROVIDERS)).toBe("gemma4:12b");
    expect(chipName("gemma4:12b", undefined, PROVIDERS)).toBe("gemma4");
  });

  it("drops the variant tag, which is most of the length", () => {
    expect(chipName("qwen2.5:7b-instruct", undefined, PROVIDERS)).toBe("qwen2.5");
    expect(chipName("anthropic:claude-opus-4-8", undefined, PROVIDERS)).toBe("claude-opus-4-8");
  });

  it("without the provider list, changes nothing rather than guessing", () => {
    expect(stripProvider("anthropic:claude-opus-4-8")).toBe("anthropic:claude-opus-4-8");
  });

  it("keeps the name and drops the vendor half of a curated label", () => {
    const labels = { "qwen2.5:7b-instruct": "qwen2.5:7b-instruct · Custom (OpenAI-compatible)" };
    expect(chipName("qwen2.5:7b-instruct", labels, PROVIDERS)).toBe("qwen2.5");
    // The menu keeps it: there, the vendor is what tells two similar names apart.
    expect(menuName("qwen2.5:7b-instruct", labels)).toBe("qwen2.5:7b-instruct · Custom (OpenAI-compatible)");
  });

  it("drops :latest with the rest of the tag", () => {
    expect(chipName("llama3.2-vision:latest", { "llama3.2-vision:latest": "llama3.2-vision:latest · Ollama" })).toBe("llama3.2-vision");
    expect(chipName("nomic-embed-text:LATEST")).toBe("nomic-embed-text");
  });

  // Two variants of one model collapse to the same chip. That is the trade, and it is
  // why the whole id is in the tooltip and every menu row keeps its full name.
  it("is deliberately ambiguous between variants, and the menu is not", () => {
    expect(chipName("qwen2.5:7b", undefined, PROVIDERS)).toBe("qwen2.5");
    expect(chipName("qwen2.5:72b", undefined, PROVIDERS)).toBe("qwen2.5");
    expect(menuName("qwen2.5:72b")).toBe("qwen2.5:72b");
  });

  it("never returns an empty chip", () => {
    expect(chipName("weird", { weird: " · Ollama" })).toBe("weird");
    expect(chipName(":only-a-tag", undefined, PROVIDERS)).toBe(":only-a-tag");
    expect(chipName("")).toBe("");
  });

  it("leaves an id with no colon alone", () => {
    expect(chipName("gpt-5.6-sol", undefined, PROVIDERS)).toBe("gpt-5.6-sol");
    expect(stripProvider(":leading", PROVIDERS)).toBe(":leading");
  });
});
