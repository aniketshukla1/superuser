---
name: project-tracker
description: Turn a plan or a pile of notes into a project tracker spreadsheet (CSV)
allowed-tools: sheet_new, write_file
---
Produce `project-tracker.csv` with these columns:
`Task,Owner,Status,Due,Blocked by,Notes`

1. Extract tasks from whatever the user gives you (notes, a doc, a thread). One row
   per task, phrased as an action starting with a verb.
2. `Status` is one of: Not started / In progress / Blocked / Done. Use "Not started"
   when it isn't stated rather than guessing.
3. Leave `Owner` and `Due` empty when unknown — do not assign work to people who
   were never mentioned.
4. Fill `Blocked by` only when a dependency is explicit in the source.
5. Finish with a two-line summary: how many tasks, and which ones look blocked or
   undated and need the user's attention.
