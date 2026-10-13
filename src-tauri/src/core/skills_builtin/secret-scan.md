---
name: secret-scan
description: Sweep the working tree and full git history for leaked credentials
allowed-tools: check_tool, git_diff, git_log, git_status, grep, list_dir, read_file, record_coverage, request_tool, run_shell, write_file
---
Find credentials that were committed. The working tree is the easy half; history is where
the real ones live.

1. **Check the tool.** `check_tool("gitleaks")`. If it is missing, `request_tool`. If that
   is declined, do the sweep by hand — and say so via `record_coverage(…, "degraded", …)`,
   because a manual sweep is genuinely narrower.
2. **Scan the working tree and the history:**
   `gitleaks detect --source . --report-path outputs/gitleaks.json --redact`
   `--redact` is not optional: it keeps the values out of the report file.
3. **If gitleaks is unavailable, sweep manually — both halves.**
   - The working tree: `.env` files, config, CI definitions, notebooks, fixtures.
   - The history: `git log -p` over the paths that ever held config, and the contents of
     any DELETED env or config file. A secret removed from HEAD but alive in history is
     exactly what this check exists to catch — checking only HEAD is not this check.
4. **Verify what is live.** A committed key that was rotated two years ago is a cleanup
   task. One that still works is an incident. Where you can tell the difference without
   using the credential, say which it is. Never authenticate with a discovered secret to
   test it.
5. **NEVER print a secret's value.** Not in your reply, not in a note, not in a commit
   message, not in the report page. Refer to it by location, kind, and when it was
   committed: "a Stripe live key in `deploy/prod.env`, added 2024-11-04, still in history".
6. **Deliver:** every finding as location · kind · first committed · still in history? ·
   rotated?, and the remediation order. Say plainly that removing a secret from HEAD does
   not fix it — the credential must be rotated, and history rewriting is a separate,
   coordinated job the user decides on.
