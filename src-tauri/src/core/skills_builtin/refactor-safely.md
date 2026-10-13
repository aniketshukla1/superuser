---
name: refactor-safely
description: Change the shape of code without changing what it does, and be able to prove it
allowed-tools: grep, list_dir, read_file, run_shell
---
A refactor that changes behaviour is not a refactor; it is an undocumented feature
change with no test and no reviewer expecting one.

1. **Establish the safety net first.** Run the tests and record what passes. If the
   code you are about to move is untested, that is the first commit — characterisation
   tests that pin what it does TODAY, bugs included. Write them against observed
   behaviour, not intended behaviour; the point is to notice change, not to be right.
2. **Say what shape you are moving toward and why**, in one sentence. "Extract the
   permission decision so it can be tested without a session." A refactor without a
   destination wanders and ends up as a diff nobody can review.
3. **One mechanical change at a time**, each leaving the suite green: rename, then
   extract, then move, then inline. Combining them makes a diff where nobody can see
   which step introduced the failure — including you.
4. **Preserve behaviour exactly, including the ugly parts.** A bug you fix mid-refactor
   is a bug that lands unannounced in a commit claiming to change nothing. Note it,
   leave it, fix it separately.
5. **Keep the reasons.** A comment explaining why the odd branch exists is the most
   valuable thing in the file and the easiest to drop while moving code. If you
   cannot tell why something is there, that is a question, not dead weight.
6. **Re-run everything and diff the behaviour you can observe** — same output, same
   files written, same queries.
7. Ship as `refactor/<what-moved>`, one theme per branch, and say in the description
   what did NOT change so a reviewer knows what to check.
