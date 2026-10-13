---
name: support-triage
description: Turn a queue of tickets into a ranked list with a reason for each rank
allowed-tools: write_file
---
Triage is not sorting by date. It is deciding, defensibly, whose problem is worst.

1. **Read for the problem, not the request.** "Can you reset my export?" is often "my
   export has been silently truncating for a month". Answering the request and missing
   the problem is how the same ticket comes back three times.
2. **Rank on impact × reach × reversibility.** Data loss, money moving wrongly, and
   lost access come first. One customer blocked outranks fifty mildly annoyed, and
   anything that cannot be undone outranks anything that can.
3. **Find the duplicates before you answer any of them.** Five tickets with one cause
   is an incident, not five tickets, and treating it as five is how a systemic failure
   gets closed five times individually.
4. **Separate what you know from what the customer reported.** Their diagnosis is a
   hint. Reproduce it, or check the logs, before it becomes a bug report — see
   `bug-report` for what engineering needs to act without asking a question.
5. **Say what happens next and when, on every ticket you touch**, including the ones
   you are not fixing. "Not now, and here is why" is an answer; silence is not.
6. **Flag the ones that should not exist.** A ticket caused by a confusing screen or a
   missing doc is a product finding — count them, name the cause, and pass it on. A
   support queue that never produces product findings is one nobody is reading.
7. Deliver `outputs/triage-<date>.md`: the ranked list with a one-line reason each, the
   duplicate clusters with a suspected common cause, and the recurring causes worth fixing.
