---
name: iac-scan
description: Scan Terraform and cloud configuration, and fix what matters in the IaC
allowed-tools: check_tool, grep, list_dir, read_file, request_tool, run_shell, table, write_file
---
Review infrastructure-as-code for the misconfigurations that actually expose something.

1. **Check the tools.** `check_tool("trivy")` and `check_tool("checkov")`. Missing ones go
   through `request_tool`, or get a manual pass recorded as `degraded`.
2. **Scan from the repo root:**
   `trivy config . --format json --output outputs/trivy-iac.json`
   `checkov -d . -o json --output-file-path outputs/`
   Run both where both are available — their rule sets do not overlap as much as you'd expect.
3. **Triage against what is actually deployed.** A permissive rule in a module nobody
   instantiates is not a finding. A `0.0.0.0/0` ingress on a bastion is a decision; the same
   on a database is an incident. Read the module graph far enough to tell which you're
   looking at, and say which.
4. **Rate** critical / high / medium / low / noise, and for each one say what an attacker
   could reach. "Public bucket" is not a finding; "public bucket holding nightly database
   dumps" is.
5. **Fix in the IaC, never in a console.** Match the module's conventions. One theme per
   branch. If a fix needs a value you do not have (an account id, a CIDR), ask rather than
   inventing one.
6. **Say what a fix will do on apply.** Some of these are disruptive — tightening a security
   group drops live connections. The user needs to know that before they merge, not after.
7. **Deliver:** findings table (severity · resource · verdict · action), the branch, and the
   apply-time warnings.
