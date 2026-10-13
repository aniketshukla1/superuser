---
name: exploratory-charter
description: Run a timeboxed exploratory session against one risk, and report what you actually saw
allowed-tools: write_file
---
Scripted tests find what someone already thought of. This is for the rest.

1. **Pick ONE risk and write the charter in a sentence**: explore <area> with <data>
   to discover <what could go wrong>. A charter covering three areas is three sessions
   pretending to be one, and the notes will be useless for all of them.
2. **Timebox it** — 45 minutes is typical. Stop when the box ends, not when you run
   out of ideas; the leftovers are the next charter.
3. **Vary deliberately, one dimension at a time**: the boundary values, the empty and
   the enormous, the wrong order, the interrupted flow, the second window, the stale
   tab, the back button. Randomly clicking is not exploration, it is a demo.
4. **Take notes as you go**, not after: what you did, what you expected, what
   happened. A surprise you did not write down is a surprise you will not reproduce.
5. **Chase surprises immediately**, but keep the original charter — note the detour,
   finish it, come back. An unfinished charter that found something is fine; one that
   silently became a different charter is not.
6. **Separate what you saw from what you conclude.** "The list showed 0 items" is an
   observation; "pagination is broken" is a theory, and often the wrong one.
7. Deliver `outputs/charter-<area>.md`: the charter, the timebox, what you tried,
   what you found with reproduction steps, what you did not reach, and the risks you
   now think are worth a scripted test.
