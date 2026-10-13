---
name: data-profile
description: Understand a dataset's shape and defects before anyone draws a conclusion from it
allowed-tools: chart, sheet_profile, write_file
---
Every wrong analysis this skill exists to prevent starts the same way: someone
charted a column before finding out what was in it.

1. **Profile before you analyse.** Row count, and for every column: type, how many
   are missing, how many are distinct, min/max, and the handful of most common values.
   `sheet_profile` does this; read its output rather than skimming it.
2. **Find the defects that change conclusions**, and name them:
   - **missing** — and whether missing means zero, unknown, or not-applicable, which
     are three different numbers,
   - **duplicates** — by what key, and whether they are genuine repeats,
   - **mixed units or formats** in one column: "1.2k" beside 1200, dates in two
     layouts, a total row hiding among the data,
   - **outliers** that are data-entry errors rather than findings.
3. **Check the grain.** One row per what? An analysis that sums a column whose rows
   are already subtotals produces a confident number that is roughly double.
4. **Say what the data cannot answer.** No date column means no trend; one region
   missing means no comparison. Naming this early prevents the chart that implies it.
5. **Never silently repair.** Every correction is stated: what you changed, how many
   rows, and why — a cleaned dataset with no record of the cleaning is not evidence.
6. Deliver `outputs/data-profile.md`: the shape, the defects with counts, the grain,
   the questions this data can and cannot answer, and every correction you made.
