import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoriesTab, readableStamps, titleParts } from "./MemoriesTab";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("titleParts", () => {
  it("separates the sync's kind from the name", () => {
    expect(titleParts("Repo: aniketshukla1/mem0")).toEqual({ kind: "Repo", name: "aniketshukla1/mem0" });
    expect(titleParts("status-note")).toEqual({ name: "status-note" });
  });

  it("does not split a sentence that happens to contain a colon", () => {
    expect(titleParts("opened PR #378 in acme/billing-api")).toEqual({ name: "opened PR #378 in acme/billing-api" });
  });
});

describe("readableStamps", () => {
  it("rewrites ISO stamps and leaves the rest of the text alone", () => {
    const out = readableStamps("- Last push: 2026-07-30T14:11:10Z\n- Stars: 0");
    expect(out).not.toContain("2026-07-30T14:11:10Z");
    expect(out).toContain("- Stars: 0");
  });
});

describe("MemoriesTab", () => {
  const ROW = {
    id: "1",
    filename: "github-repo-mem0.md",
    title: "Repo: aniketshukla1/mem0",
    category: "GitHub",
    tags: ["github"],
    summary: "Universal memory layer for AI Agents",
    date: "2026-08-05T00:00:00Z",
    weight: 1,
    origin: "external",
  };

  function stub() {
    const calls: string[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string, init?: RequestInit) => {
        calls.push(`${init?.method ?? "GET"} ${url}`);
        if (String(url).includes("/v1/memory/ledger")) return { ok: true, json: async () => ({ initialized: false }) } as Response;
        if (init?.method === "DELETE") return { ok: true, json: async () => ({ ok: true }) } as Response;
        if (String(url).includes("github-repo-mem0")) {
          return {
            ok: true,
            json: async () => ({ ok: true, memory: { ...ROW, body: "**aniketshukla1/mem0**\n\n- Stars: 0" } }),
          } as Response;
        }
        return { ok: true, json: async () => ({ memories: [ROW] }) } as Response;
      }),
    );
    return calls;
  }

  it("opens the first memory instead of an empty reader", async () => {
    stub();
    render(<MemoriesTab />);
    await waitFor(() => expect(document.querySelector(".mem-detail-title")?.textContent).toBe("aniketshukla1/mem0"));
    expect(screen.queryByText("Pick a memory to read it.")).toBeNull();
  });

  it("renders a memory's body as text, not as markdown source", async () => {
    stub();
    render(<MemoriesTab />);
    await waitFor(() => expect(document.querySelector(".mem-body strong")?.textContent).toBe("aniketshukla1/mem0"));
    expect(document.querySelector(".mem-body")?.textContent).not.toContain("**");
  });

  it("offers a way to fill an empty vault, and no search box over nothing", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) =>
        String(url).includes("/v1/memory/ledger")
          ? ({ ok: true, json: async () => ({ initialized: false }) } as Response)
          : ({ ok: true, json: async () => ({ memories: [] }) } as Response),
      ),
    );
    render(<MemoriesTab />);
    expect(await screen.findByRole("link", { name: /Connect an app/ })).toBeTruthy();
    expect(screen.queryByRole("searchbox")).toBeNull();
  });

  it("asks before forgetting", async () => {
    const calls = stub();
    render(<MemoriesTab />);
    fireEvent.click(await screen.findByText("Forget…"));
    expect(calls.some((c) => c.startsWith("DELETE"))).toBe(false);
    fireEvent.click(screen.getByTestId("mem-forget-confirm"));
    await waitFor(() => expect(calls.some((c) => c.startsWith("DELETE"))).toBe(true));
  });
});
