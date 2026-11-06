---
name: semgrep-review
description: Scan with semgrep, judge each finding in context, and fix the real ones
allowed-tools: check_tool, grep, list_dir, read_file, record_coverage, request_tool, run_shell, table, write_file
---
A semgrep pass that ends in fixes, not in a raw report.

1. **Make sure semgrep is there.** Call `check_tool("semgrep")`. When it is absent, call
   `request_tool("semgrep", …)` instead of dropping the pass. If the user turns the install
   down, review by hand (routes and handlers, authentication and session code, any query
   assembled from strings, deserialisation, outbound requests shaped by user input) and
   then call `record_coverage("Static analysis for code vulnerabilities", "degraded", …)`,
   so the report shows this pass was manual and narrower.
2. **Run it from the repository root:**
   `semgrep scan --config auto --json --quiet -o outputs/semgrep.json`
   When the repository ships its own rules (`.semgrep.yml` or `semgrep.yml`), use those
   instead of `--config auto`.
3. **Judge each finding yourself** instead of repeating the report.
   - Open the flagged code with enough of its surroundings to tell whether it can be reached.
   - Decide whether an attacker or the system itself supplies the input, and whether
     something earlier already guards it.
   - Give it a rating (critical, high, medium, low or noise) and one line saying why.
   The community rules can't see everything (a query built through the project's own
   database wrapper, say), so read the risky code even after a clean scan.
4. **Fix the real ones, worst first.**
   - Write the fix the codebase's way: its validators, its escaping helpers, its
     parameterised queries. Read the neighbouring code first.
   - Where the test setup allows it, write a test that only passes once the fix is in.
   - One branch per theme; never one large mixed diff.
5. **Explain every "noise" verdict**: the input is constant, the code is dead, the framework
   escapes it already. Don't drop a finding quietly, and don't add an ignore rule to silence
   semgrep unless the user agrees.
6. **Hand over** a table of the findings (severity · location · verdict · action) and
   the branches with the fixes. When the repository has no semgrep config, offer a starter `.semgrep.yml`
   pinned to the rulesets that turned out to matter here.
