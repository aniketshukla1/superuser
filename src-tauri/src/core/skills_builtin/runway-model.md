---
name: runway-model
description: Model how long the money lasts, with the assumptions visible and testable
allowed-tools: write_file
---
Every runway number is a set of assumptions wearing a single figure. The assumptions
are the deliverable; the figure is a summary of them.

1. **Start from actuals, not from a plan.** Last three to six months of real cash out,
   by category. A model built on the budget tells you how long the budget lasts.
2. **Separate committed from discretionary.** Payroll, rent and contracts behave
   differently from tooling and travel, and the difference is exactly what someone
   will want to flex when the number is bad.
3. **State every assumption as a line someone can argue with**: headcount changes and
   when, revenue growth rate, payment timing, one-offs. An assumption inside a formula
   is an assumption nobody can challenge.
4. **Model three cases** — as-is, a downside, and a plan-adjusted — and say what
   distinguishes them in one line each. A single number invites false precision; three
   invite a decision.
5. **Give the date, not just the months.** "Cash runs out in week of 14 April 2027" is
   actionable in a way "11 months" is not, and it exposes an off-by-one that months
   hide.
6. **Name the sensitivities**: which single assumption moves the date most. That is
   what the reader should watch, and usually it is not the one they are watching.
7. Deliver `outputs/runway.md` plus the workbook, with every assumption in a labelled
   cell rather than typed into a formula — a model nobody can re-run is a screenshot.
