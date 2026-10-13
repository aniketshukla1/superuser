import { fetch, httpBase, jsonOf } from "./neverRejectingJson";
import { jsonPost } from "./skills";

// ---- Telegram (two-way messaging) ----------------------------------------------

/** The email bot: new email from listed senders gets a reply drafted
 *  by the Email Assistant, and every draft waits in the Inbox for approval. */
export interface EmailBotSetup {
  /** Email (IMAP) is connected. */
  connected: boolean;
  address: string;
  inbound: boolean;
  allowed: string;
  allowed_count: number;
  /** Really running: on, connected, someone listed, and the assistant still read-only. */
  live: boolean;
  /** Why it can't run when it's on, in one sentence — or null. */
  problem: string | null;
  drafted_today: number;
  daily_cap: number;
}

/** This setup and the four after it (expenses, Chief of Staff, Bug Reproduction, Product
 *  Performance) come back as `null` unless the body has the field its Settings section reads
 *  first. Each section renders nothing for `null`, and that is the point: a body of the wrong
 *  shape (an error object, a core that predates the endpoint) used to throw inside the
 *  section's render and take the whole Settings page down with it. */
export async function getEmailBot(): Promise<EmailBotSetup | null> {
  const res = await fetch(`${httpBase()}/v1/email-bot`);
  const j = res.ok ? await jsonOf(res) : {};
  return typeof j.allowed === "string" ? (j as EmailBotSetup) : null;
}

export async function setEmailBot(body: {
  email_inbound?: boolean;
  email_allowed?: string;
}): Promise<EmailBotSetup & { ok?: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/email-bot`, jsonPost(body));
  return res.json();
}

/** The Expense Manager's intake and policy: receipts that arrive
 *  by email are read by the model on this machine into a ledger it summarizes. */
export interface ExpensesSetup {
  /** Email (IMAP) is connected. */
  connected: boolean;
  intake: boolean;
  policy: string;
  receipts: number;
  this_month: number;
  read_today: number;
  daily_cap: number;
  /** Where the ledger lives, a CSV any spreadsheet opens. */
  ledger: string;
  /** The weekly summary routine exists. */
  routine: boolean;
}

export async function getExpenses(): Promise<ExpensesSetup | null> {
  const res = await fetch(`${httpBase()}/v1/expenses`);
  const j = res.ok ? await jsonOf(res) : {};
  return typeof j.policy === "string" ? (j as ExpensesSetup) : null;
}

export async function setExpenses(body: {
  expense_intake?: boolean;
  expense_policy?: string;
}): Promise<ExpensesSetup & { ok?: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/expenses`, jsonPost(body));
  return res.json();
}

/** Add the weekly summary routine — once; `created: false` when it already exists. */
export async function addExpenseRoutine(): Promise<{ ok: boolean; created?: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/expenses/routine`, jsonPost({}));
  return res.json();
}

/** The Chief of Staff: a digest of what came in since yesterday,
 *  mapped to the priorities set here. */
export interface ChiefSetup {
  priorities: string;
  /** What feeds the digest: each source, with how many items it brought in the last week. */
  sources: { source: string; items: number }[];
  /** The weekday-morning digest routine exists. */
  routine: boolean;
}

export async function getChief(): Promise<ChiefSetup | null> {
  const res = await fetch(`${httpBase()}/v1/chief-of-staff`);
  const j = res.ok ? await jsonOf(res) : {};
  return typeof j.priorities === "string" ? { ...(j as ChiefSetup), sources: Array.isArray(j.sources) ? j.sources : [] } : null;
}

export async function setChief(body: { cos_priorities: string }): Promise<ChiefSetup & { ok?: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/chief-of-staff`, jsonPost(body));
  return res.json();
}

/** Add the weekday-morning digest — once; `created: false` when it already exists. */
export async function addChiefRoutine(): Promise<{ ok: boolean; created?: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/chief-of-staff/routine`, jsonPost({}));
  return res.json();
}

/** Bug Reproduction: reproduces a bug report on staging, and
 *  nowhere else, in its own browser. */
export interface BugReproSetup {
  /** The staging address its browser is locked to. Empty: none set, so it opens nothing. */
  staging: string;
  /** The assistants' browser is switched on (Settings ▸ Browser). */
  browser: boolean;
  /** Staging is a dev server on this Mac, which needs `loopback` allowed. */
  on_this_mac: boolean;
  loopback: boolean;
  /** Local-only mode: nothing but this Mac's own servers is reachable. */
  local_only: boolean;
}

export async function getBugRepro(): Promise<BugReproSetup | null> {
  const res = await fetch(`${httpBase()}/v1/bug-repro`);
  const j = res.ok ? await jsonOf(res) : {};
  return typeof j.staging === "string" ? (j as BugReproSetup) : null;
}

export async function setBugRepro(body: { repro_staging: string }): Promise<BugReproSetup & { ok?: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/bug-repro`, jsonPost(body));
  return res.json();
}

/** Product Performance: investigates a slowdown in the
 *  connected Prometheus, live and read-only. */
export interface PerfSetup {
  /** Prometheus (or Thanos, Mimir, VictoriaMetrics, Grafana Cloud) is connected. */
  connected: boolean;
  /** What it connected as, e.g. "Prometheus 3.13.1 · prometheus:9090". */
  account: string | null;
  /** The signals to watch: `Name: PromQL`, one a line. */
  signals: string;
  count: number;
  problems: string[];
  /** The weekly health report routine exists. */
  routine: boolean;
}

export interface PerfCheck {
  name: string;
  ok: boolean;
  series?: number;
  error?: string;
}

export async function getPerf(): Promise<PerfSetup | null> {
  const res = await fetch(`${httpBase()}/v1/perf`);
  const j = res.ok ? await jsonOf(res) : {};
  return typeof j.signals === "string" ? { ...(j as PerfSetup), problems: Array.isArray(j.problems) ? j.problems : [] } : null;
}

export async function setPerf(body: { perf_signals: string }): Promise<PerfSetup & { ok?: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/perf`, jsonPost(body));
  return res.json();
}

/** Run each saved signal once, now. */
export async function checkPerf(): Promise<{ ok: boolean; results?: PerfCheck[]; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/perf/check`, jsonPost({}));
  return res.json();
}

/** Add the weekly health report — once; `created: false` when it already exists. */
export async function addPerfRoutine(): Promise<{ ok: boolean; created?: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/perf/routine`, jsonPost({}));
  return res.json();
}

export interface TelegramSetup {
  connected: boolean;
  chat_id: string;
  allowed: string;
  inbound: boolean;
  /** Inbound is only really on when a chat is allowlisted too. */
  inbound_live: boolean;
}

export async function getTelegram(): Promise<TelegramSetup | null> {
  const res = await fetch(`${httpBase()}/v1/telegram`);
  return res.ok ? res.json() : null;
}
