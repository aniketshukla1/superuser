---
name: semgrep-review
description: Run a semgrep scan and turn the findings into triaged, contextual fixes
allowed-tools: check_tool, grep, list_dir, read_file, record_coverage, request_tool, run_shell, table, write_file
---
Run a static-analysis pass with semgrep and own the findings end to end.

1. **Check the tool.** `check_tool("semgrep")`. If it is missing, `request_tool("semgrep", …)`
   rather than skipping the pass. If the user declines, do a targeted manual review — read
   the routes and handlers, the auth and session code, every query built by string
   concatenation, deserialization, and outbound requests built from user input — then
   `record_coverage("Static analysis for code vulnerabilities", "degraded", …)` so the
   report says the pass was manual and therefore narrower.
2. **Scan from the repo root:**
   `semgrep scan --config auto --json --quiet -o outputs/semgrep.json`
   Use `--config auto` unless the repo carries its own rules (`.semgrep.yml`, `semgrep.yml`) —
   prefer the repo's own configuration when it exists.
3. **Triage EVERY finding.** Do not echo the raw report.
   - Read the flagged code and enough around it to judge reachability.
   - Is the tainted input attacker-controlled or internal? Is there an upstream guard?
   - Rate critical / high / medium / low / noise, with a one-line justification each.
   Community rules miss whole classes — SQL built through a project's own DB wrapper, for
   one — so reading the code is worth doing even when the scan runs clean.
4. **Fix what is real**, highest severity first.
   - Match the codebase's own conventions: its validation helpers, its escaping utilities,
     its parameterized-query style. Read neighbouring code before writing.
   - Add or extend a test that fails without the fix, where the harness makes that reasonable.
   - Group fixes by theme, one branch per theme. Never a giant mixed diff.
5. **For findings you judge noise, say WHY** — constant input, dead code, the framework
   already escapes it. Never silently drop one, and never add an ignore rule to quiet the
   scanner without agreement.
6. **Deliver:** a findings table (severity · location · verdict · action) and the fix
   branches. If the repo has no semgrep config, offer to add a starter `.semgrep.yml`
   pinned to the rulesets that actually mattered here.
