---
name: product-spec
description: Specify a change so it can be built and tested without a meeting
allowed-tools: write_file
---
A spec is finished when someone else could build the thing and someone else could
tell whether they had.

1. **Write the problem before the solution**, in terms of what someone cannot do
   today and what it costs them. A spec that opens with a feature has skipped the
   step where you find out it is the wrong feature.
2. **Say who has this problem and how often.** "Users want" is nobody. If you cannot
   name the situation it happens in, the spec is a guess and should say so.
3. **Define done as observable behaviour**, in the same form as a test: given this
   state, this action, this result. This is the section engineering builds against
   and QA verifies — vague here means arguments later, in both directions.
4. **Write the edge cases yourself.** Empty, first-run, permission denied, offline,
   too long, already exists, concurrent. Leaving these to be discovered is how a
   two-day change becomes a two-week one.
5. **State what is explicitly OUT.** The neighbouring feature you are not building is
   the single most useful line in a spec, and the one most often missing.
6. **Name the open questions with an owner and a decision date.** An unmarked open
   question is a hole someone fills silently with their own answer.
7. Deliver `outputs/spec-<slug>.md`: problem, who and when, behaviour, edges,
   explicitly out, open questions, and how you will know it worked after shipping.
