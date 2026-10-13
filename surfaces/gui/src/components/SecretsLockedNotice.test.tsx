import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { SecretsLockedNotice } from "./SecretsLockedNotice";

// Saved keys that won't open are said out loud, with the reason and the way out; the way out
// keeps the file; and a store that opens shows nothing at all.

let posted: string[] = [];

beforeEach(() => {
  posted = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => {
      posted.push(String(url));
      return { ok: true, json: async () => ({ ok: true, kept_at: "/data/secrets.locked-1.json" }) } as Response;
    }),
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("SecretsLockedNotice", () => {
  it("shows nothing while the keys open", () => {
    for (const status of [undefined, "encrypted", "empty", "plain"]) {
      const { container } = render(<SecretsLockedNotice status={status} onResolved={() => {}} />);
      expect(container.textContent).toBe("");
      cleanup();
    }
  });

  it("says why, and sets the locked file aside when asked", async () => {
    const resolved = vi.fn();
    render(<SecretsLockedNotice status="locked: the key that opens it is missing" onResolved={resolved} />);
    expect(screen.getByRole("alert").textContent).toContain("The key that opens it is missing.");
    expect(screen.getByRole("alert").textContent).toContain("kept, not deleted");
    fireEvent.click(screen.getByTestId("secrets-set-aside"));
    await waitFor(() => expect(resolved).toHaveBeenCalled());
    expect(posted[0]).toContain("/v1/secrets/set-aside");
  });

  it("can be put away for now", () => {
    render(<SecretsLockedNotice status="locked: the key that opens it is missing" onResolved={() => {}} />);
    fireEvent.click(screen.getByTestId("secrets-not-now"));
    expect(screen.queryByRole("alert")).toBeNull();
    expect(posted).toHaveLength(0);
  });
});
