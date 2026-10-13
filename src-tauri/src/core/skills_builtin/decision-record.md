---
name: decision-record
description: Record a technical decision with the alternatives, so the next person can reopen it honestly
allowed-tools: write_file
---
Code shows what was decided. It never shows what else was considered, which is
exactly what someone needs a year later when the constraints have moved.

1. **State the decision in one sentence**, in the present tense: "Sessions record
   their own working folder rather than deriving it."
2. **Write the context as it actually was** — the constraint, the deadline, the thing
   you did not know yet. A record that reads as though the answer was obvious is a
   record that will be reversed by someone who thinks you did not consider the
   alternative.
3. **List the alternatives you genuinely weighed**, each with why it lost. An
   alternatives section containing only straw men is worse than none: it makes a
   reader trust the reasoning less, not more.
4. **Say what this costs.** Every real decision has a price — a slower path, a
   migration, a thing that is now harder. A record with no downside is marketing.
5. **Name what would change your mind.** "If we ever need X, revisit this." This is
   what makes a record reopenable instead of scripture.
6. **Date it and say who decided.** A decision with no date cannot be weighed against
   what was known at the time.
7. Deliver `outputs/decisions/<NNNN>-<slug>.md`, numbered in sequence. Never edit a
   superseded one — write the new record and link back, because the history of the
   thinking is the artifact.
