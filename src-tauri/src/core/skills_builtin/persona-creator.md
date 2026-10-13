---
name: persona-creator
description: Make a new assistant with the user — check nobody already does the job, then put one up for approval
allowed-tools: list_personas, read_persona, list_persona_skills, save_persona, ask_user
---
An assistant is a standing role: instructions that apply to every conversation it has, plus
what it is allowed to use. Both halves are the work.

1. **Check nobody already does the job.** `list_personas`, then `read_persona` on the
   closest one or two. If one already fits, say so and stop. If one nearly fits, say what
   would have to change about it — changing a role beats adding a near-duplicate that
   splits the person's attention.
2. **Ask what it is for, not what it should say.** At most three questions in a message:
   what work it takes on, what it must never do, and what a good answer from it looks like.
3. **Write the standing role in the second person**, as what this assistant does: how it
   starts, what it checks, what it refuses, and how it hands back. Keep it about the role —
   anything true of every assistant is already in the app.
4. **Give it as little as it needs.** What it may use is what it may do, so add a
   capability only when a step in the role needs it, and pick the permission mode the same
   way: `interactive` unless there is a reason. `list_persona_skills` shows the skills it
   could be given; none means it gets whatever is installed.
5. **Propose it once** with `save_persona`. The user sees the whole thing and presses a
   card; nothing is written until they do. Problems come back to you — fix them and call it
   again rather than passing them on.

Afterwards, say what it is for, what it was given, and what it deliberately was not.
