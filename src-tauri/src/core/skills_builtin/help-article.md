---
name: help-article
description: Answer one question a user actually has, in the words they used to ask it
allowed-tools: write_file
---
A help article is read by someone who is stuck, mid-task, and impatient. It gets one
job.

1. **Title it with the user's question, in their words.** "Why is my export empty?"
   finds its reader; "Export configuration reference" does not. Use the phrasing from
   real tickets, not the internal name for the feature.
2. **Put the answer first.** The fix, in the first two lines, before the explanation.
   Someone who only reads the top of the page should already be unstuck.
3. **Write the steps as they appear on screen.** Exact button labels, in order, one
   action per step. If a step needs a permission the reader may not have, say so at
   that step, not in a preamble they skipped.
4. **Cover the one wrong turn people actually take.** Ticket history says which it is.
   One well-chosen "if you see X instead" beats a troubleshooting section of eight.
5. **Say when this article is the wrong page**, and link the right one. Sending the
   reader onward in one line is better than a near-miss answer they try anyway.
6. **Keep it to one question.** An article that answers three is found for none of them.
   Split it.
7. Verify every step against the product as it is now, and mark any you could not check.
   Deliver `outputs/help-<slug>.md` with the title, the answer, the steps, and the
   unchecked items named.
