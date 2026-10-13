---
name: code-review
description: Review a change for defects that survive to production, and say which ones matter
allowed-tools: git_diff, git_log, git_status, grep, list_dir, read_file, run_shell, write_file
---
Review a diff. Not the whole codebase, not the style — the change.

1. **Get the actual diff first.** `git diff main...HEAD`, or the range you were given.
   Reviewing from a description is reviewing something that may not exist.
2. **Read what it touches before judging it.** A change is only correct relative to
   the code around it: the caller's assumptions, the existing error handling, the
   convention the file already follows. A comment that says "this looks wrong" about
   code that matches its neighbours is a comment about the codebase, not the change.
3. **Hunt in this order**, because it is the order of what survives to production:
   - **Correctness under the inputs nobody typed**: empty, absent, duplicate,
     concurrent, out of order, very large. Name the concrete input and the wrong
     output — "empty list → panics at line 40", not "handle edge cases".
   - **The half that was missed**: a write with no read updated, a new field nothing
     serializes, an error path that returns success, a branch with no test.
   - **State that outlives the change**: a migration with no rollback, a cache nobody
     invalidates, a default that silently applies to existing rows.
   - **Then** simplification and duplication — real, but not first.
4. **Every finding needs a failure scenario.** If you cannot say what breaks and when,
   you have a preference, not a finding. Say so and move on.
5. **Rank and cut.** Three findings someone acts on beat twelve they skim. Lead with
   the one you would block on; say plainly when there is nothing to block on.
6. **Deliver `outputs/code-review.md`**: a verdict line, then findings as
   file:line · what breaks · why · suggested fix. Say what you did NOT review and
   why — a review whose coverage is unstated reads as a review of everything.
