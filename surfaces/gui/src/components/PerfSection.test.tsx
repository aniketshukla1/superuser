import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PerfSection } from "./PerfSection";

// Settings ▸ General ▸ Product Performance — the connection, the signals, and the weekly report.

type Call = { url: string; method: string; body: any };

const SIGNALS = "Checkout p95: histogram_quantile(0.95, sum by (le) (rate(http_seconds_bucket[5m])))";
const BASE = {
  connected: true,
  account: "Prometheus 3.13.1 · prom:9090",
  signals: SIGNALS,
  count: 1,
  problems: [] as string[],
  routine: false,
};

function stubFetch(routes: { match: string; method?: string; json: object }[]) {
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const method = (init?.method || "GET").toUpperCase();
      calls.push({ url, method, body: init?.body ? JSON.parse(String(init.body)) : undefined });
      const hit = routes.find((r) => url.includes(r.match) && (!r.method || r.method === method));
      return { ok: true, json: async () => hit?.json ?? {} } as Response;
    }),
  );
  return calls;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("PerfSection", () => {
  it("says what it reads, or where to connect it", async () => {
    stubFetch([{ match: "/v1/perf", method: "GET", json: BASE }]);
    render(<PerfSection />);
    expect((await screen.findByTestId("perf-connection")).textContent).toBe("Reading Prometheus 3.13.1 · prom:9090.");
    cleanup();
    stubFetch([{ match: "/v1/perf", method: "GET", json: { ...BASE, connected: false, account: null } }]);
    render(<PerfSection />);
    expect((await screen.findByTestId("perf-connection")).textContent).toContain("Connectors ▸ Prometheus");
    expect(screen.queryByText("Check signals")).toBeNull();
  });

  it("saves the signals, trimmed, and shows why a line was refused", async () => {
    const calls = stubFetch([
      { match: "/v1/perf", method: "POST", json: { ok: false, error: "Line 1 isn't a signal — write it as a name, a colon and a space, then the query." } },
      { match: "/v1/perf", method: "GET", json: { ...BASE, signals: "", count: 0 } },
    ]);
    render(<PerfSection />);
    fireEvent.change(await screen.findByLabelText("Signals to watch"), { target: { value: "  p95 without a colon  " } });
    fireEvent.click(screen.getByText("Save"));
    await waitFor(() => expect(calls.find((c) => c.method === "POST")?.body).toEqual({ perf_signals: "p95 without a colon" }));
    expect((await screen.findByRole("status")).textContent).toContain("Line 1 isn't a signal");
  });

  it("checks the saved signals and says which work", async () => {
    stubFetch([
      {
        match: "/v1/perf/check",
        method: "POST",
        json: {
          ok: true,
          results: [
            { name: "Checkout p95", ok: true, series: 3 },
            { name: "Errors", ok: true, series: 0 },
            { name: "Queue", ok: false, error: "the query didn't parse: unexpected end of input" },
          ],
        },
      },
      { match: "/v1/perf", method: "GET", json: { ...BASE, count: 3 } },
    ]);
    render(<PerfSection />);
    fireEvent.click(await screen.findByText("Check signals"));
    const list = await screen.findByTestId("perf-checks");
    expect(list.textContent).toContain("Checkout p95: works (3 series)");
    expect(list.textContent).toContain("Errors: no series right now");
    expect(list.textContent).toContain("Queue: the query didn't parse");
  });

  it("won't check signals that aren't saved yet", async () => {
    stubFetch([{ match: "/v1/perf", method: "GET", json: BASE }]);
    render(<PerfSection />);
    fireEvent.change(await screen.findByLabelText("Signals to watch"), { target: { value: SIGNALS + "\nErrors: up" } });
    expect(screen.queryByText("Check signals")).toBeNull();
  });

  it("adds the weekly report once, then says it runs", async () => {
    stubFetch([
      { match: "/v1/perf/routine", method: "POST", json: { ok: true, created: true } },
      { match: "/v1/perf", method: "GET", json: BASE },
    ]);
    render(<PerfSection />);
    fireEvent.click(await screen.findByText("Add a weekly health report"));
    expect((await screen.findByRole("status")).textContent).toContain("every Monday at 9:30");
    cleanup();
    stubFetch([{ match: "/v1/perf", method: "GET", json: { ...BASE, routine: true } }]);
    render(<PerfSection />);
    expect(await screen.findByText("A health report runs Mondays at 9:30.")).toBeTruthy();
  });
});
