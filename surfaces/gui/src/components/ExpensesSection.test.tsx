import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ExpensesSection } from "./ExpensesSection";

// Settings ▸ General ▸ Expenses — the Expense Manager's intake switch, policy and routine.

type Call = { url: string; method: string; body: any };

const BASE = {
  connected: true,
  intake: false,
  policy: "",
  receipts: 0,
  this_month: 0,
  read_today: 0,
  daily_cap: 60,
  ledger: "/data/expenses/ledger.csv",
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

describe("ExpensesSection", () => {
  it("offers the policy even before email is connected, and asks for the connection for receipts", async () => {
    stubFetch([{ match: "/v1/expenses", method: "GET", json: { ...BASE, connected: false } }]);
    render(<ExpensesSection />);
    expect(await screen.findByText(/Connect your email first to log receipts/)).toBeTruthy();
    expect(screen.queryByRole("switch")).toBeNull();
    expect(screen.getByLabelText("Your expense policy")).toBeTruthy();
  });

  it("switching intake on saves the switch with the policy", async () => {
    const calls = stubFetch([
      { match: "/v1/expenses", method: "POST", json: { ...BASE, intake: true, policy: "Meals up to 50 USD a day.", ok: true } },
      { match: "/v1/expenses", method: "GET", json: BASE },
    ]);
    render(<ExpensesSection />);
    fireEvent.change(await screen.findByLabelText("Your expense policy"), { target: { value: " Meals up to 50 USD a day. " } });
    fireEvent.click(screen.getByRole("switch", { name: "Log receipts from email" }));
    await waitFor(() => {
      const post = calls.find((c) => c.method === "POST" && c.url.endsWith("/v1/expenses"));
      expect(post?.body).toEqual({ expense_policy: "Meals up to 50 USD a day.", expense_intake: true });
    });
  });

  it("adds the weekly summary once, and then says it runs", async () => {
    const calls = stubFetch([
      { match: "/v1/expenses/routine", method: "POST", json: { ok: true, created: true } },
      { match: "/v1/expenses", method: "GET", json: { ...BASE, receipts: 3, this_month: 2 } },
    ]);
    render(<ExpensesSection />);
    expect((await screen.findByTestId("expenses-count")).textContent).toContain("2 receipts this month, 3 in all.");
    fireEvent.click(screen.getByText("Add a weekly summary"));
    expect((await screen.findByRole("status")).textContent).toContain("every Monday at 9:00");
    expect(calls.some((c) => c.method === "POST" && c.url.endsWith("/v1/expenses/routine"))).toBe(true);
  });

  it("says the routine is set up instead of offering it twice", async () => {
    stubFetch([{ match: "/v1/expenses", method: "GET", json: { ...BASE, routine: true } }]);
    render(<ExpensesSection />);
    expect(await screen.findByText("A weekly summary runs Mondays at 9:00.")).toBeTruthy();
    expect(screen.queryByText("Add a weekly summary")).toBeNull();
  });
});
