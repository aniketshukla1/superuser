---
name: test-backfill
description: Add tests to untested code, starting where the absence actually costs something
allowed-tools: grep, list_dir, read_file, run_shell, write_file
---
Coverage is not the goal — a suite that is 90% lines and misses every branch that
matters is worse than an honest 40%, because it is trusted.

1. **Find where the absence hurts**, not where it is easiest: the code that handles
   money, permissions, deletion, concurrency, or anything whose failure is silent.
   A getter with no test costs nothing; a permission check with no test is the one
   that quietly inverts.
2. **Read the code and write down what it PROMISES** before writing an assertion.
   Tests that restate the implementation ("calls foo then bar") break on every
   refactor and catch nothing; tests that state the promise ("a denied user gets no
   rows") survive rewrites and catch the bug.
3. **Start with the ugly inputs**, since the happy path is usually the one that was
   manually tried: empty, absent, duplicate, enormous, concurrent, out of order,
   and the malformed file someone will eventually have on disk.
4. **Make each test fail first.** Break the code, watch it go red, put it back. A test
   that has never failed is a test that may assert nothing — and this happens more
   often than anyone expects.
5. **Name the test after the property, not the function.** `a_denied_user_gets_no_rows`
   tells a future reader what broke; `test_get_rows_2` does not.
6. **Isolate**: no shared temp directory, no shared clock, no ordering assumption.
   Concurrent tests sharing a name is the classic source of a suite that fails once
   in ten runs and erodes trust in all of it.
7. Deliver the tests plus `outputs/test-plan.md`: what you covered, what you
   deliberately did not, and any behaviour you found that looks wrong but that you
   pinned as-is rather than fixing.
