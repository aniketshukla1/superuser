---
name: security-fix-pr
description: Take security findings that have been triaged and land them as one focused fix branch
allowed-tools: grep, list_dir, read_file, run_shell
---
Ship the fix as something a colleague can actually review.

1. **One theme per branch.** All the XSS in one, all the missing authorization checks in
   another. A branch mixing twelve unrelated security fixes gets rubber-stamped, which
   defeats the point of review — and if one fix has to be reverted it takes the rest with it.
2. **Branch from the current head**, named for the theme: `security/escape-user-html`.
3. **Write the fix in the codebase's own idiom.** Read the neighbouring code first. If the
   project has a validation helper, use it; if it has an escaping convention, follow it. A
   fix that fights the surrounding style will be reverted by the next person who touches
   the file.
4. **Write the test that fails without the fix.** A security fix without a regression test is
   a security fix with an expiry date. Where the harness makes that unreasonable, say so
   rather than skipping it silently.
5. **Run the tests and report the real result.** If they fail, fix or say so — never
   describe an unverified change as done.
6. **Write the description for a reviewer who was not in this conversation:**
   - What the vulnerability was, and what an attacker could have done.
   - Why this fix closes it.
   - What you deliberately did not change, and why.
   - The finding it closes, by location — never by pasting a secret or an exploit payload.
7. **If a connector is set up for the host, offer to open the PR.** Do not push or open one
   without saying so first: an unannounced security branch on a public remote is a
   disclosure. Ask before it leaves the machine.
