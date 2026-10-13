---
name: test-plan
description: Decide what to test before a release, and what you are consciously not testing
allowed-tools: git_diff, git_log, git_status, grep, list_dir, read_file, run_shell, write_file
---
A test plan's value is the second half: what you decided to leave. A plan that
implies everything was covered is how a release ships believing it was.

1. **Establish what changed.** `git log` since the last release, the diff, the
   tickets. You cannot risk-rank a release you have not read.
2. **Rank by blast radius, not by how new it is.** What loses data, takes money,
   blocks a login, or cannot be undone goes first; a new setting nobody has found yet
   goes last, however recently it landed.
3. **For each area, write the cases as observable outcomes**: given this state and
   this action, this is what the user sees. "Test the login" is not a case; "a wrong
   password three times shows the lockout message and does not send a reset email" is.
4. **Include the states people forget**: first run with no data, an account mid-way
   through setup, a slow network, a second tab, a session that expired while the form
   was open.
5. **Say what is automated and what needs hands.** An area listed without saying which
   is an area that will be assumed automated and tested by nobody.
6. **Write down what you are NOT testing and why** — out of scope, low risk, no
   environment. This is the part people skip, and it is the part that makes the plan
   honest.
7. Deliver `outputs/test-plan.md`: the changes, the ranked areas, the cases, and the
   explicit not-testing list.
