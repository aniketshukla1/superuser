// The Memory-search model picker. It exists because this setting had no control at
// all: a chat model in the embedding slot makes Ollama answer 501, recall falls back
// to keyword matching, and the only way back was hand-editing settings.json.
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";

vi.mock("../api", () => ({ setEmbedModel: vi.fn(async () => ({ ok: true })) }));

import { EmbeddingModelCard } from "./ManageTabs";
import { setEmbedModel } from "../api";

const base = {
  provider: "ollama",
  model: "gemma4:12b",
  models: ["gemma4:12b", "nomic-embed-text:latest"],
  has_key: false,
  model_ready: true,
  source: "store" as const,
  onboarded: true,
  surfaces: {} as any,
  scratch_base: "",
  secrets_path: "",
};

const render_ = (extra: Record<string, unknown>) =>
  render(<EmbeddingModelCard settings={{ ...base, ...extra } as any} onChanged={() => {}} />);

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("EmbeddingModelCard", () => {
  it("offers the installed embedding models and saves the pick", async () => {
    render_({ embed_model: "nomic-embed-text", embed_models: ["nomic-embed-text", "mxbai-embed-large"] });
    const select = screen.getByTestId("embed-model-select") as HTMLSelectElement;
    expect(select.value).toBe("nomic-embed-text");
    fireEvent.change(select, { target: { value: "mxbai-embed-large" } });
    await waitFor(() => expect(setEmbedModel).toHaveBeenCalledWith("mxbai-embed-large"));
  });

  // The exact state this machine was in for months, and the whole point of the card.
  it("keeps a non-embedding model visible and warns that search has degraded", () => {
    render_({ embed_model: "llama3.2:latest", embed_models: ["nomic-embed-text"] });
    const select = screen.getByTestId("embed-model-select") as HTMLSelectElement;
    // Shown, not silently swapped — the picker must not misreport what is in use.
    expect(select.value).toBe("llama3.2:latest");
    expect(screen.getByTestId("embed-model-warning").textContent).toMatch(/keyword matching/);
  });

  it("says nothing is wrong when the configured model is one of the installed ones", () => {
    render_({ embed_model: "nomic-embed-text", embed_models: ["nomic-embed-text"] });
    expect(screen.queryByTestId("embed-model-warning")).toBeNull();
  });

  // The real shape on this machine: the setting is stored untagged and Ollama lists
  // it tagged. Comparing literally called a working setup broken.
  it("matches the installed model regardless of its :latest tag", () => {
    render_({ embed_model: "nomic-embed-text", embed_models: ["nomic-embed-text:latest"] });
    expect(screen.queryByTestId("embed-model-warning")).toBeNull();
    // ...does not list the same model twice...
    const select = screen.getByTestId("embed-model-select") as HTMLSelectElement;
    expect([...select.options].length).toBe(1);
    // ...and shows the entry actually serving, rather than falling back to option 0
    // because the stored value matched no option.
    expect(select.value).toBe("nomic-embed-text:latest");
  });

  it("tells you how to install one when none is present", () => {
    render_({ embed_model: "", embed_models: [] });
    expect(screen.queryByTestId("embed-model-select")).toBeNull();
    expect(screen.getByText(/ollama pull nomic-embed-text/)).toBeTruthy();
  });

  it("surfaces a refusal from the server instead of appearing to have saved", async () => {
    (setEmbedModel as any).mockResolvedValueOnce({ ok: false, error: "'x' isn't an embedding model" });
    render_({ embed_model: "nomic-embed-text", embed_models: ["nomic-embed-text", "mxbai-embed-large"] });
    fireEvent.change(screen.getByTestId("embed-model-select"), { target: { value: "mxbai-embed-large" } });
    await waitFor(() => expect(screen.getByText(/isn't an embedding model/)).toBeTruthy());
  });

  // An older server sends neither field; an empty picker would be worse than none.
  it("hides itself against a server that doesn't report the field", () => {
    render_({});
    expect(screen.queryByTestId("embed-model")).toBeNull();
  });
});
