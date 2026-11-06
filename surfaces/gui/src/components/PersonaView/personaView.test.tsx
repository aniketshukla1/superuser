import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PersonaView } from "../PersonaView";
import { CONNECTORS, DETAIL, stubFetch } from "./PersonaView.helpers";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
describe("PersonaView", () => {
  it("gives it a browser of its own, and takes it back", async () => {
    const calls = stubFetch([
      { match: "/v1/personas/ops", method: "GET", json: DETAIL },
      { match: "/v1/personas/ops", method: "POST", json: { ok: true } },
      { match: "/v1/connectors", method: "GET", json: CONNECTORS },
    ]);
    render(<PersonaView personaId="ops" />);
    const box = (await screen.findByTestId("persona-browser-toggle")) as HTMLInputElement;
    expect(box.checked).toBe(false);
    fireEvent.click(box);
    await waitFor(() => expect(calls.some((c) => c.method === "POST" && c.body?.browser === true)).toBe(true));
    expect(screen.getByText("browser")).toBeTruthy(); // its chip, beside the other capabilities
    fireEvent.click(box);
    await waitFor(() => expect(calls.some((c) => c.method === "POST" && c.body?.browser === false)).toBe(true));
    expect(screen.queryByText("browser")).toBeNull();
  });

  it("says a refused browser change in the section and keeps the page", async () => {
    stubFetch([
      { match: "/v1/personas/ops", method: "GET", json: DETAIL },
      { match: "/v1/personas/ops", method: "POST", json: { ok: false, error: "unknown assistant: ops" } },
      { match: "/v1/connectors", method: "GET", json: CONNECTORS },
    ]);
    render(<PersonaView personaId="ops" />);
    const box = (await screen.findByTestId("persona-browser-toggle")) as HTMLInputElement;
    fireEvent.click(box);
    expect((await screen.findByTestId("persona-browser-error")).textContent).toContain("unknown assistant");
    expect(box.checked).toBe(false);
    expect(screen.getByText("Ops Assistant")).toBeTruthy();
  });

  it("offers no browser to an assistant that runs on a remote agent", async () => {
    stubFetch([
      { match: "/v1/personas/ops", method: "GET", json: { ...DETAIL, remote: "https://agent.example.com/run" } },
      { match: "/v1/connectors", method: "GET", json: CONNECTORS },
    ]);
    render(<PersonaView personaId="ops" />);
    await screen.findByText("Ops Assistant");
    expect(screen.queryByTestId("persona-browser")).toBeNull();
  });

  it("renders the persona detail (identity, tools, recommends + connect state) from the endpoint", async () => {
    stubFetch([
      { match: "/v1/personas/ops", method: "GET", json: DETAIL },
      { match: "/v1/connectors", method: "GET", json: CONNECTORS },
    ]);
    render(<PersonaView personaId="ops" />);

    expect(await screen.findByText("Ops Assistant")).toBeTruthy();
    expect(screen.getByText("Incidents and runbooks")).toBeTruthy();
    expect(screen.getByText("A careful, methodical operations engineer.")).toBeTruthy();
    // tools rendered as chips
    expect(screen.getByText("shell")).toBeTruthy();
    // a connected recommend shows "connected"; an unconnected one offers Connect/Add
    expect(screen.getByText("connected")).toBeTruthy();
    expect(screen.getByText("Connect")).toBeTruthy(); // datadog (core, not connected)
    expect(screen.getByText("Add")).toBeTruthy(); // filesystem (mcp, not connected)
    // defaults footer
    // A property of the WORK, which is true whatever models are installed —
    // where a list of ids was advice a local-first user could not act on.
    expect(screen.getByText(/needs a capable one/)).toBeTruthy();
  });

  it("toggling a default connection POSTs /connections and applies the returned defaults", async () => {
    const calls = stubFetch([
      { match: "/v1/personas/ops", method: "GET", json: DETAIL },
      { match: "/v1/connectors", method: "GET", json: CONNECTORS },
      {
        match: "/v1/personas/ops/connections",
        method: "POST",
        json: {
          ok: true,
          default_connections: [
            { connector: "slack", enabled: false, connected: true },
            { connector: "datadog", enabled: false, connected: false },
          ],
        },
      },
    ]);
    render(<PersonaView personaId="ops" />);
    await screen.findByText("Ops Assistant");

    // Switches in DOM order: [0] persona Enable, then the default-connection toggles. Slack is the
    // checked+enabled default; datadog is disabled (not connected). Target the last checked+enabled
    // switch — the Slack default — and flip it off.
    const switches = screen.getAllByRole("switch");
    const candidates = switches.filter(
      (s) => s.getAttribute("aria-checked") === "true" && !(s as HTMLButtonElement).disabled,
    );
    fireEvent.click(candidates[candidates.length - 1]);

    await waitFor(() => {
      const post = calls.find(
        (c) => c.method === "POST" && c.url.includes("/v1/personas/ops/connections"),
      );
      expect(post).toBeTruthy();
      expect(post!.body).toMatchObject({ connector: "slack", enabled: false });
    });
  });

  it("toggling Enable POSTs /enable", async () => {
    const calls = stubFetch([
      { match: "/v1/personas/ops", method: "GET", json: DETAIL },
      { match: "/v1/connectors", method: "GET", json: CONNECTORS },
      { match: "/v1/personas/ops/enable", method: "POST", json: { ok: true } },
    ]);
    render(<PersonaView personaId="ops" />);
    await screen.findByText("Ops Assistant");

    // The enable switch is the first one in DOM order (identity header).
    const enableToggle = screen.getAllByRole("switch")[0];
    fireEvent.click(enableToggle);

    await waitFor(() => {
      const post = calls.find((c) => c.method === "POST" && c.url.includes("/v1/personas/ops/enable"));
      expect(post).toBeTruthy();
      expect(post!.body).toMatchObject({ enabled: false });
    });
  });
});

describe("who an assistant may ask", () => {
  const OTHERS = [
    { id: "ops", name: "Ops Assistant", tagline: "Incidents and runbooks", enabled: true },
    { id: "analyst", name: "Analyst", tagline: "Reads the numbers", enabled: true },
    { id: "retired", name: "Retired", tagline: "", enabled: false },
  ];

  it("offers every other assistant, never itself or a disabled one", async () => {
    stubFetch([
      { match: "/v1/personas/ops", json: { ...DETAIL, asks: ["analyst"] } },
      { match: "/v1/personas", json: { personas: OTHERS } },
    ]);
    render(<PersonaView personaId="ops" />);
    const analyst = (await screen.findByTestId("asks-analyst")) as HTMLInputElement;
    expect(analyst.checked).toBe(true);
    expect(screen.queryByTestId("asks-ops")).toBeNull();
    expect(screen.queryByTestId("asks-retired")).toBeNull();
  });

  it("a tick is a grant, and it goes back if the server says no", async () => {
    const calls = stubFetch([
      { match: "/v1/personas/ops", method: "POST", json: { ok: false, error: "nope" } },
      { match: "/v1/personas/ops", json: { ...DETAIL, asks: [] } },
      { match: "/v1/personas", json: { personas: OTHERS } },
    ]);
    render(<PersonaView personaId="ops" />);
    const analyst = (await screen.findByTestId("asks-analyst")) as HTMLInputElement;
    fireEvent.click(analyst);
    await waitFor(() => expect(calls.some((c) => c.method === "POST" && c.body?.asks?.includes("analyst"))).toBe(true));
    // The page says what the server said rather than keeping a tick that means nothing —
    // the same way its mode and team toggles report a refusal.
    await screen.findByText("nope");
  });
});
