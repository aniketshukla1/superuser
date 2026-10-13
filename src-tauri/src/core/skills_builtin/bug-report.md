---
name: bug-report
description: Write a bug someone can reproduce, judge and fix without asking you anything
allowed-tools: write_file
---
The measure of a bug report is whether a stranger can act on it without a
conversation. Most cannot, and the fix is almost always the same three missing things.

1. **Reproduce it yourself first**, twice, from a clean state. A report you could not
   reproduce is a hypothesis, and saying so is more useful than a confident guess.
2. **Lead with the one-line effect**, in the user's terms: "Deleting a disabled
   assistant hides every other disabled one until you switch tabs." Not the cause,
   not the file — what someone experiences.
3. **Then the three things every report needs and most lack**:
   - **Steps**, numbered, from a known starting state, with the exact data used.
   - **Expected vs actual**, both stated. "It doesn't work" is neither.
   - **Environment**: version or commit, OS, and whatever else varies here.
4. **Attach the evidence**, not a description of it: the error text, the log line, the
   screenshot, the failing request and its response.
5. **Say how often**, honestly: every time, or one in five, or once. "Intermittent"
   with no ratio is the difference between a race and a fluke, and it decides who
   picks it up.
6. **Separate severity from priority.** How bad it is when it happens, and how soon it
   should be fixed, are different judgements — conflating them is how a rare
   catastrophe outranks a constant annoyance forever, or the reverse.
7. Deliver `outputs/bug-<slug>.md`. Do not include a proposed fix unless you looked at
   the code; a guess in the report becomes the thing everyone investigates first.
