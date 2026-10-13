import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { StandingInstructions } from "./StandingInstructions";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

function stubFetch(saved = "") {
  const posts: any[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const reply = (v: any) => ({ ok: true, json: async () => v }) as Response;
      if (String(url).endsWith("/v1/settings/standing-instructions")) {
        const body = JSON.parse(String(init!.body));
        posts.push(body);
        return reply({ ok: true, standing_instructions: body.text.trim(), max: 20 });
      }
      return reply({ standing_instructions: saved, standing_instructions_max: 20 });
    }),
  );
  return posts;
}

describe("StandingInstructions", () => {
  it("saves what was typed, and says which wins when it disagrees with a role", async () => {
    const posts = stubFetch();
    render(<StandingInstructions />);
    const box = (await screen.findByTestId("standing-instructions-text")) as HTMLTextAreaElement;
    expect(screen.getByTestId("standing-instructions").textContent).toContain("the role decides what it does");

    fireEvent.change(box, { target: { value: "  British spelling. " } });
    fireEvent.click(screen.getByTestId("standing-instructions-save"));
    await waitFor(() => expect(posts).toEqual([{ text: "  British spelling. " }]));
    expect(await screen.findByText("Saved")).toBeTruthy();
    expect(box.value).toBe("British spelling.");
  });

  it("won't save past the limit, and says by how much", async () => {
    stubFetch();
    render(<StandingInstructions />);
    const box = await screen.findByTestId("standing-instructions-text");
    fireEvent.change(box, { target: { value: "x".repeat(25) } });
    expect(screen.getByText("5 characters over the 20 limit")).toBeTruthy();
    expect((screen.getByTestId("standing-instructions-save") as HTMLButtonElement).disabled).toBe(true);
  });
});
