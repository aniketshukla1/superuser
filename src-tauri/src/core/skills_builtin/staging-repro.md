---
name: staging-repro
description: Reproduce a bug report on staging in your own browser — exact steps, screenshots, console notes and a minimal test case
allowed-tools: browser_navigate, browser_snapshot, browser_read, browser_click, browser_type, browser_key, browser_scroll, browser_request_secret, browser_console, browser_screenshot, read_file, write_file
---
Reproduce the bug on staging, then write down exactly what happened. Do not fix anything, and do not
touch production — your browser only opens the staging address, and that is the point.

1. **Write the claim.** From the report: what should happen, what happens instead, and where. A report
   that doesn't say what should happen isn't reproducible yet — ask for that before starting.
2. **Start clean.** A fresh test account, never a customer's and never a real person's. When a page
   wants a password or a code, use `browser_request_secret` on that field: the person types it into the
   page and you never see it. Never ask for a password in the conversation. When signing in happens on
   another site (single sign-on), your browser won't show it to you: use `browser_request_help` and the
   person signs in themselves.
3. **Clear the console, then follow the steps exactly** as a person would, with the report's words for
   buttons and fields. Call `browser_console` once before the step you're checking so it starts empty.
4. **At the moment it goes wrong:** `browser_screenshot` with what it shows, and `browser_console` for
   what the page reported — console errors, uncaught exceptions, requests that failed.
5. **Do it again from a clean start.** Twice the same is a reproduction; once in three is an
   intermittent bug and is reported as "1 of 3", never as reproduced or not.
6. **Cut it down.** The fewest steps that still fail become a minimal test case: a Playwright test in
   `outputs/repro.spec.ts` for anything in the page, or a `curl` sequence in `outputs/repro.sh` for an
   API. It points at the staging address, never production, and carries no real password.
7. **Write `outputs/repro.md`:**
   - **Status** — Reproduced, Not reproduced, Intermittent (n of m), or Couldn't try, and why.
   - **Environment** — the browser line `browser_console` gave, the staging address, the test account's
     role (never its password).
   - **Steps** — numbered, exact labels, one action each.
   - **Expected** and **Actual** — one or two sentences each.
   - **Screenshots** — what each one shows, in order.
   - **Console and network** — quoted as reported, with the request that failed and its status.
   - **Minimal test case** — the file, and how to run it.
   - **Ruled out** — when it didn't reproduce: every variation you tried, so the next person doesn't.

Posting the result to the issue tracker happens only if asked, through its connected app, which asks
before it posts.
