---
name: safe-upgrade-pr
description: Ship the smallest test-verified upgrade that clears an advisory
allowed-tools: grep, list_dir, read_file, run_shell
---
Upgrade the minimum needed, and prove it still works.

1. **Smallest bump that clears it.** Patch over minor, minor over major. `npm audit fix
   --force` and its equivalents are not this skill — they take major bumps nobody reviewed
   and will break things you then have to debug under time pressure.
2. **Read the changelog between the current and target version.** Say what you found: a
   silent behaviour change matters more than the version number does. If the fix requires a
   major bump with breaking changes, stop and put the choice to the user with the actual
   cost — do not take it on your own.
3. **One branch per theme**, named for it: `deps/upgrade-lodash-cve-2024-1234`. A lockfile
   PR mixing twelve unrelated bumps cannot be reviewed and will be merged unread.
4. **Regenerate the lockfile properly** — the package manager's own command, never a hand
   edit. A hand-edited lockfile is a supply-chain incident waiting to happen.
5. **Run the test suite. Report the real result.**
   - Pass: say so, and say which suite ran.
   - Fail: say which tests, and whether the failure is the upgrade or was already there —
     check by running them on the base commit.
   - No test suite: say that plainly. An unverified upgrade is a change, not a fix, and the
     user needs to know which one they are merging.
6. **Re-run the scanner** to confirm the advisory is actually closed. An upgrade that was
   supposed to fix it and did not is a common outcome, and finding that out after merge is
   the expensive way.
7. **Write the description for a reviewer:** the advisory, the version change, what the
   changelog said, the test result, and anything you could not verify.
