import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { StallCard } from "./SettingsView";

// The card saves on blur, not per keystroke: the server floors a limit at ten seconds,
// so typing "60" one digit at a time would otherwise pass through a saved "10". And it
// shows what the server KEPT, which is not always what was typed.

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

function stubFetch() {
  const posts: any[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const reply = (v: any) => ({ ok: true, json: async () => v }) as Response;
      if (String(url).endsWith("/v1/settings/stall")) {
        const body = JSON.parse(String(init!.body));
        posts.push(body);
        const clamp = (v: number) => (v === 0 ? 0 : Math.max(10, Math.min(3600, v)));
        return reply({
          ok: true,
          stall_seconds: body.stall_seconds === undefined ? 60 : clamp(body.stall_seconds),
          stall_seconds_local: body.stall_seconds_local === undefined ? 300 : clamp(body.stall_seconds_local),
        });
      }
      return reply({ stall_seconds: 60, stall_seconds_local: 300 });
    }),
  );
  return posts;
}

describe("StallCard", () => {
  it("saves once, on blur, and shows the limit the server kept", async () => {
    const posts = stubFetch();
    render(<StallCard />);
    const cloud = (await screen.findByTestId("stall-stall_seconds")) as HTMLInputElement;
    expect(cloud.value).toBe("60");

    fireEvent.change(cloud, { target: { value: "5" } });
    expect(posts, "typing saves nothing").toEqual([]);
    fireEvent.blur(cloud, { target: { value: "5" } });
    await waitFor(() => expect(posts).toEqual([{ stall_seconds: 5 }]));
    await waitFor(() => expect(cloud.value).toBe("10"));
  });

  it("0 reads as off", async () => {
    stubFetch();
    render(<StallCard />);
    const local = (await screen.findByTestId("stall-stall_seconds_local")) as HTMLInputElement;
    fireEvent.change(local, { target: { value: "0" } });
    expect(screen.getByTestId("stall-card").textContent).toContain("off");
  });
});
