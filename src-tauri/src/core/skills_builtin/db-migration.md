---
name: db-migration
description: Change a schema without a window where the running code and the data disagree
allowed-tools: grep, list_dir, read_file, run_shell, table, write_file
---
The danger in a migration is not the SQL. It is the minutes — or the release cycle —
where old code is running against new data, or the reverse.

1. **Establish what runs against this schema right now**, including anything you do
   not deploy: a background job, an older client, a read replica, a report.
2. **Split it into steps that are each safe alone.** The expand/contract shape:
   - **Expand**: add the new column nullable, add the new table, backfill in batches.
     Old code ignores it; new code tolerates its absence.
   - **Migrate**: deploy code that writes BOTH and reads the new.
   - **Contract**: only once nothing reads the old — drop it.
   Doing these in one step is what causes the outage, and it always looks unnecessary
   until the rollback.
3. **Write the rollback before the migration.** A step you cannot undo needs saying
   out loud, and a backfill that cannot be re-run is a step you get one attempt at.
4. **Batch every backfill** with a bound and a resume point. A single `UPDATE` over a
   large table takes a lock, and the lock is the outage.
5. **Test on a copy with realistic volume.** A migration that is instant on 100 rows
   and takes 40 minutes on 100 million is not the same migration.
6. Deliver `outputs/migration-plan.md`: the steps in order, which deploy each belongs
   to, the rollback for each, the backfill's batch size and estimated duration, and
   what breaks if the steps are run out of order.
