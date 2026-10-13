import { NOW } from "./FromSource";

// ---- Brain ▸ Memories: plain markdown files, by where they came from ----
export const memory = (category: string, title: string, summary: string, days: number, origin: "internal" | "external" = "external") => ({
  id: `${category}-${title}`.toLowerCase().replace(/[^a-z0-9]+/g, "-"),
  filename: `${category}-${title}`.toLowerCase().replace(/[^a-z0-9]+/g, "-") + ".md",
  title,
  category,
  tags: [category.toLowerCase()],
  summary,
  date: new Date((NOW - days * 86_400) * 1000).toISOString(),
  weight: 1,
  origin,
});
export const MEMORIES = [
  memory("GitHub", "PR: Inline the tax-service call in checkout confirm", "Merged into checkout 2.41.0. Adds a synchronous call on the confirm path.", 1),
  memory("GitHub", "Issue: Flaky test checkout_confirm_retries", "Fails about one run in six; suspected race on the order lock.", 2),
  memory("GitHub", "PR: Add index on orders.created_at", "Approved, waiting on a migration window.", 4),
  memory("GitHub", "Repo: northwind/checkout", "Go service. Owners: payments team. Deploys Tuesdays and Thursdays.", 9),
  memory("Email", "Email: Renewal quote from Dana at Acme", "Quote expires Friday. 12 months, 40 seats.", 1),
  memory("Email", "Email: Interview loop for the backend role", "Two loops need a panel member by Thursday.", 2),
  memory("Email", "Email: Receipt — team offsite dinner", "$412.80, 8 people. Logged to the expense ledger.", 3),
  memory("Calendar", "Event: Checkout 2.41 release review", "Tomorrow 10:00, with payments and platform.", 0),
  memory("Calendar", "Event: Q4 planning", "Thursday 14:00. Memo due the day before.", 1),
  memory("Notes", "Priorities for this week", "Close the Acme renewal. Ship 2.41 without a latency regression. Hire two backend engineers.", 1, "internal"),
  memory("Notes", "How I like status updates", "Three bullets, decisions first, no preamble.", 12, "internal"),
];
