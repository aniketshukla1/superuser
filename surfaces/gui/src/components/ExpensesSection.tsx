import { useEffect, useState } from "react";
import { addExpenseRoutine, getExpenses, setExpenses, type ExpensesSetup } from "../api";
import { CARD } from "../ui";

// The Expense Manager's settings.
//
// Receipts that arrive by email are read by the model on this machine into a ledger, and the
// Expense Manager builds summaries from it: totals that add up, and what breaks your policy,
// each exception citing its rule. It never sends anything or changes a receipt.

const INPUT =
  "w-full min-w-0 px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
const BTN = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
const BTN_BORDERED = "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40";

export function ExpensesSection() {
  const [ex, setEx] = useState<ExpensesSetup | null>(null);
  const [policy, setPolicy] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const load = () =>
    getExpenses().then((e) => {
      setEx(e);
      if (e) setPolicy(e.policy);
    });
  useEffect(() => {
    load();
  }, []);

  const save = async (extra: Partial<{ expense_intake: boolean }> = {}) => {
    setBusy(true);
    setNotice(null);
    const res = await setExpenses({ expense_policy: policy.trim(), ...extra });
    setBusy(false);
    if (res.ok === false) {
      setNotice(res.error ?? "Couldn't save that.");
      return;
    }
    await load();
    setNotice("Saved.");
  };

  const addRoutine = async () => {
    setBusy(true);
    setNotice(null);
    const res = await addExpenseRoutine();
    setBusy(false);
    if (!res.ok) {
      setNotice(res.error ?? "Couldn't add the routine.");
      return;
    }
    await load();
    setNotice(res.created ? "Added: the Expense Manager summarizes last week every Monday at 9:00." : "It's already set up.");
  };

  if (!ex) return null;

  return (
    <section className={`${CARD} p-4 mt-6`} data-testid="expenses">
      <div className="text-md font-medium">Expenses</div>
      <p className="text-sm text-muted mt-1 mb-3 leading-prose">
        Receipts that arrive in your email go into a ledger, and the Expense Manager turns it into
        summaries whose totals add up, with anything that breaks your policy called out. It never
        sends anything or changes a receipt.
      </p>

      {!ex.connected ? (
        <div className="text-sm text-muted mb-3">
          Connect your email first to log receipts: sidebar ▸ More ▸ Connectors ▸ Email (IMAP).
        </div>
      ) : (
        <label className="flex items-start gap-2 text-sm mb-3">
          <input
            type="checkbox"
            role="switch"
            className="mt-0.5"
            aria-label="Log receipts from email"
            checked={ex.intake}
            disabled={busy}
            onChange={(e) => save({ expense_intake: e.target.checked })}
          />
          <span>
            <span className="font-medium">Log receipts from email</span>
            <span className="block text-xs text-muted leading-prose">
              Only mail that arrives after you switch this on. Receipts are read by the model on
              this machine — never a cloud one.
            </span>
          </span>
        </label>
      )}

      <label className="text-sm font-medium text-ink" htmlFor="expense-policy">
        Your expense policy
      </label>
      <textarea
        id="expense-policy"
        className={`${INPUT} mt-1 mb-1 min-h-[72px]`}
        placeholder={"Meals up to 50 USD a day.\nEconomy flights only.\nNo alcohol."}
        value={policy}
        onChange={(e) => setPolicy(e.target.value)}
      />
      <div className="text-xs text-muted mb-3">
        One rule a line. Every exception the Expense Manager flags cites the rule it breaks.
      </div>

      <div className="text-xs text-muted mb-3" data-testid="expenses-count">
        {ex.receipts === 0
          ? "No receipts logged yet."
          : `${ex.this_month} receipt${ex.this_month === 1 ? "" : "s"} this month, ${ex.receipts} in all.`}{" "}
        The ledger is a spreadsheet file: <span className="font-mono break-all">{ex.ledger}</span>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <button className={BTN} disabled={busy} onClick={() => save()}>
          {busy ? "Saving…" : "Save"}
        </button>
        {ex.routine ? (
          <span className="text-xs text-muted">A weekly summary runs Mondays at 9:00.</span>
        ) : (
          <button className={BTN_BORDERED} disabled={busy} onClick={addRoutine}>
            Add a weekly summary
          </button>
        )}
        {notice ? (
          <span className="text-xs text-muted" role="status">
            {notice}
          </span>
        ) : null}
      </div>
    </section>
  );
}
