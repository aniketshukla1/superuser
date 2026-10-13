---
name: dependency-audit
description: Audit lockfiles for known vulnerabilities and separate exploitable from theoretical
allowed-tools: check_tool, grep, list_dir, read_file, request_tool, run_shell, write_file
---
Find the advisories that actually matter for this project.

1. **Check the tools** for the ecosystems present: `check_tool("osv-scanner")`, plus
   `pip-audit` or trivy as they fit. `npm audit` ships with npm. Missing ones go through
   `request_tool`, or get recorded as `degraded` — and name the ecosystem left unchecked,
   because an unscanned ecosystem reads exactly like a clean one.
2. **Scan every lockfile, not just the obvious one.** A monorepo has several, and the one
   nobody remembers is the one that is three years stale:
   `osv-scanner --lockfile <path> --format json --output outputs/osv-<name>.json`
3. **For each advisory that survives a first pass, answer three questions with evidence:**
   - Direct dependency or transitive? Transitive means the fix may not be yours to make.
   - Is the vulnerable function actually reached from this codebase? Search for the call.
   - Runtime or dev-only? A dev-only advisory is real but is not production risk.
4. **Rate: exploitable here / theoretical here / not applicable**, one line of reasoning
   each. A version number is not an analysis. Resist the temptation to report severity as
   the advisory states it — CVSS describes the vulnerability, not your exposure to it.
5. **Group by fix.** Ten advisories often close with two upgrades. Report it that way: the
   user wants the number of actions, not the number of rows.
6. **Deliver:** package · advisory · direct/transitive · reachable? · verdict · fix, with
   the exploitable ones first and a one-paragraph summary a non-specialist can act on.
