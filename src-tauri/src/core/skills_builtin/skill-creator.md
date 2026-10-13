---
name: skill-creator
description: Write a new skill with the user — look first, interview, draft, rehearse, then put it up for approval
allowed-tools: list_skills, read_skill, list_skill_tools, save_skill, ask_user
---
A skill is instructions every later run will follow. Write it the way you would write a
procedure for somebody else to carry out while you are away.

1. **Look before you name it.** Call `list_skills`, and `read_skill` on anything close. If
   one already covers the job, say so and stop — a second half-overlapping skill makes both
   harder to pick between. If one nearly covers it, propose changing that one instead.
2. **Interview, and keep it short.** Ask at most three questions in a message, and only the
   ones whose answers would change what you write: what triggers this, what the finished
   thing looks like, what has gone wrong before. Don't ask what you can read.
3. **Write it in the imperative, as numbered steps.** Each step is something to do, in
   order, with the check that says it worked. Name the failure it prevents where you know
   one — a step whose reason is stated survives contact with a hurry.
4. **Declare the tools it needs.** `list_skill_tools` gives the names. Declare what the
   steps actually use and nothing else.
5. **Rehearse it once, in your head, against a realistic request**, and fix what the
   rehearsal shows before anyone else reads it. Then call `save_skill` — the user sees the
   whole thing and presses a card; nothing is written until they do. If it comes back with
   problems, they are yours to fix, not theirs to read: fix them and call it again.

Afterwards, say in one or two lines what it does and when it will be used, and what you
left out.

**Never write a skill that works around a refusal.** If something was refused — by a
boundary, by a mode, by an approval — a skill that routes around it is the same act with
more steps. Say that plainly instead.
