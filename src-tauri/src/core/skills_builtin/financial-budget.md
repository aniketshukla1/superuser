---
name: financial-budget
description: Build a budget and cash-flow tracker as a spreadsheet (CSV)
allowed-tools: sheet_new, write_file
---
Produce a real budget file — not a description of one.

1. Ask for anything you genuinely need and cannot infer: the period (month/quarter),
   the currency, and where the numbers come from (a file, a connector, or the user).
   Ask once, in one message; don't interrogate.
2. Write `budget.csv` with these columns, in this order:
   `Category,Subcategory,Planned,Actual,Variance,Notes`
   - Group rows by category (Income first, then fixed costs, then variable).
   - `Variance` is `Actual - Planned`; leave it blank when Actual is unknown.
3. Add a `TOTAL` row per category and a final `NET` row.
4. Write a short `budget-notes.md` beside it: the assumptions you made, anything you
   had to guess, and what the user should check.
5. Tell the user the file path and the two or three numbers that actually matter.

Never invent figures. If a number is unknown, leave the cell empty and list it in
the notes as an open question — a plausible-looking fabricated budget is worse than
an incomplete one.
