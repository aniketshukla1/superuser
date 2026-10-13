---
name: variance-report
description: Explain the gap between plan and actual, with the causes rather than the sizes
allowed-tools: table, write_file
---
A variance report that lists differences is a spreadsheet. One that explains them is
the job.

1. **Pin both sides first**: which plan version, which actuals, which period, which
   currency. Half of all variance arguments are two people comparing different
   baselines and neither saying which.
2. **Profile before subtracting.** A missing month reads as a −100% variance; a
   duplicate invoice reads as an overspend. Check the grain and the completeness
   before a single difference is calculated — see `data-profile`.
3. **Compute variance in absolute AND percentage terms**, and lead with whichever is
   material. A 300% overrun on £40 is noise; a 4% overrun on £2m is the story, and a
   report sorted by percentage buries it.
4. **Explain the top movers, do not just rank them.** For each: what happened, and
   whether it is timing (it will reverse), volume, price, or a one-off. A variance with
   no cause attached is a question forwarded to someone else.
5. **Separate timing from real.** An invoice landing a week late is not an overspend,
   and treating it as one is how a team gets asked to cut something that was already
   on plan.
6. **Say what you could not explain.** An unexplained residual, named with its size, is
   more useful than a confident story that covers it.
7. Deliver `outputs/variance-<period>.md`: the baselines, the table, the explained
   movers with causes, the timing/real split, and the residual.
