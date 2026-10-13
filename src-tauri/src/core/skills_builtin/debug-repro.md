---
name: debug-repro
description: Reproduce a bug and isolate its cause before changing any code
allowed-tools: git_diff, git_log, git_status, grep, list_dir, read_file, run_shell, write_file
---
Find the cause. Do not fix anything yet.

The rule this exists to enforce: **a fix applied before a reproduction is a guess**,
and a guess that makes the symptom disappear is worse than no fix, because now
nobody will look again.

1. **Write down the claim** exactly as reported, and what "working" would look like.
   Vague reports ("it's slow", "sometimes fails") are not yet bugs — turn them into a
   statement that can be false.
2. **Reproduce it.** Smallest input, fewest steps, most deterministic. Record the
   exact command and the exact output in `outputs/repro.md`. If you cannot reproduce
   it, say so plainly and stop — an unreproduced bug is a hypothesis, and the
   honest deliverable is the list of things you ruled out.
3. **Bisect the surface**, not the code: does it happen with a different input? a
   different account? a fresh state directory? on the base commit? Each answer
   halves the search, and `git bisect` is the tool when the answer is "it used to
   work".
4. **Read the failing path** and form ONE hypothesis you can falsify. Then falsify
   it — add a log line, run the function directly, print the value. A hypothesis you
   only confirm is a hypothesis you chose.
5. **State the cause in one sentence**, with the evidence that proves it: this input
   reaches this line, which does this, which is wrong because of that.
6. **Then, and only then, fix.** Add the test that fails without the fix FIRST — if
   it does not fail, you have not understood the bug and the fix is decoration.
7. Deliver `outputs/repro.md`: the reproduction, the cause with evidence, the fix,
   and the test that now covers it.
