import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { RemoteAgentSection } from "./RemoteAgentSection";

// Where an assistant runs: the section says what a remote agent is NOT governed by before
// anyone points one at it; a test runs against what is typed, unsaved; a refused address
// is said in the section, not by blanking the page; and the token never comes back.

let posts: { url: string; body: any }[] = [];
let replies: Record<string, any> = {};

beforeEach(() => {
  posts = [];
  replies = {};
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const body = init?.body ? JSON.parse(String(init.body)) : undefined;
      posts.push({ url: String(url), body });
      const key = Object.keys(replies).find((k) => String(url).endsWith(k));
      return { ok: true, json: async () => (key ? replies[key] : { ok: true }) } as Response;
    }),
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("RemoteAgentSection", () => {
  it("runs here until someone opens it, and says what a remote agent escapes", () => {
    render(<RemoteAgentSection personaId="assistant" name="Assistant" remote={null} tokenSet={false} onSaved={() => {}} />);
    expect(screen.getByText(/Here, on this machine/)).toBeTruthy();
    fireEvent.click(screen.getByTestId("remote-open"));
    expect(screen.getByText(/don't reach it, and its answers count as outside content/)).toBeTruthy();
  });

  it("tests the typed address before anything is saved", async () => {
    replies["/remote/test"] = { ok: true, message: "It answered: “Hello!”" };
    render(<RemoteAgentSection personaId="assistant" name="Assistant" remote={null} tokenSet={false} onSaved={() => {}} />);
    fireEvent.click(screen.getByTestId("remote-open"));
    fireEvent.change(screen.getByTestId("remote-endpoint"), { target: { value: " https://agent.example.com/run " } });
    fireEvent.click(screen.getByTestId("remote-test"));
    await waitFor(() => expect(screen.getByTestId("remote-status").textContent).toContain("It answered"));
    expect(posts).toHaveLength(1);
    expect(posts[0].url).toContain("/v1/personas/assistant/remote/test");
    expect(posts[0].body).toEqual({ endpoint: "https://agent.example.com/run" });
  });

  it("says a refused address in the section and keeps what was typed", async () => {
    replies["/v1/personas/assistant"] = { ok: false, error: "Not saved: that address is plain http." };
    const saved = vi.fn();
    render(<RemoteAgentSection personaId="assistant" name="Assistant" remote={null} tokenSet={false} onSaved={saved} />);
    fireEvent.click(screen.getByTestId("remote-open"));
    fireEvent.change(screen.getByTestId("remote-endpoint"), { target: { value: "http://agent.example.com/run" } });
    fireEvent.click(screen.getByTestId("remote-save"));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("plain http"));
    expect((screen.getByTestId("remote-endpoint") as HTMLInputElement).value).toBe("http://agent.example.com/run");
    expect(saved).not.toHaveBeenCalled();
  });

  it("saves the token once and never shows it again", async () => {
    const saved = vi.fn();
    render(<RemoteAgentSection personaId="assistant" name="Assistant" remote={null} tokenSet={false} onSaved={saved} />);
    fireEvent.click(screen.getByTestId("remote-open"));
    fireEvent.change(screen.getByTestId("remote-endpoint"), { target: { value: "https://agent.example.com/run" } });
    fireEvent.change(screen.getByTestId("remote-token"), { target: { value: "tok-123" } });
    fireEvent.click(screen.getByTestId("remote-save"));
    await waitFor(() => expect(saved).toHaveBeenCalledWith("https://agent.example.com/run", true));
    expect(posts[0].body).toEqual({ remote: "https://agent.example.com/run", remote_token: "tok-123" });
    expect((screen.getByTestId("remote-token") as HTMLInputElement).value).toBe("");
  });

  it("brings a remote assistant back here", async () => {
    const saved = vi.fn();
    render(
      <RemoteAgentSection personaId="assistant" name="Assistant" remote="https://agent.example.com/run" tokenSet onSaved={saved} />,
    );
    expect((screen.getByTestId("remote-token") as HTMLInputElement).placeholder).toContain("saved");
    fireEvent.click(screen.getByTestId("remote-clear"));
    await waitFor(() => expect(saved).toHaveBeenCalledWith(null, false));
    expect(posts[0].body).toEqual({ remote: "" });
    expect(screen.getByText(/Here, on this machine/)).toBeTruthy();
  });
});
