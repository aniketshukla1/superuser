---
name: analysis-brief
description: Answer a question with data, showing the working and the numbers you did not like
allowed-tools: chart, write_file
---
The failure mode is not a wrong calculation. It is a true number that answers a
different question than the one asked.

1. **Restate the question so it can be answered wrong.** "Are we growing?" is not
   answerable; "did revenue in Q3 exceed Q2, per region" is. If the restatement is not
   what they meant, that is worth finding out before the work, not after.
2. **Profile the data first** — see `data-profile`. Do not skip this because the file
   looks tidy; tidy files are where the total row hides.
3. **Compute the answer, and then compute it a second way.** A different grouping, a
   spot-check against three raw rows. Two methods agreeing is evidence; one method is
   a hypothesis with a number attached.
4. **State the answer in one sentence, first**, with the number and its units and its
   period. Then the working underneath, so a reader can stop early or dig.
5. **Show the numbers that complicate it.** The region that fell while the total rose,
   the month that is half-reported. An analysis that only supports its own headline is
   advocacy, and the omission is always found later by someone else.
6. **Charts only where the shape IS the point** — a trend, a distribution, a
   comparison. Three numbers belong in a sentence, and every chart says what the
   reader should take from it.
7. Deliver `outputs/analysis-brief.md`: the question as you understood it, the answer,
   the working, the complications, and what you would need to answer it better.
