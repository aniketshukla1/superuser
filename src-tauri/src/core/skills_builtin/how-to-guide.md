---
name: how-to-guide
description: Write a guide that gets someone to a working result, tested by following it yourself
allowed-tools: write_file
---
Documentation fails in one specific way: it is written by someone who already has
the thing working. Every step they no longer notice is a step the reader gets stuck
on.

1. **Name the outcome in the title**, as a task: "Connect a GitHub account", not
   "GitHub integration". A reader arrives with a goal, not a topic.
2. **State the starting point and the finish line.** What must already be true, and
   how they will know it worked. A guide with no success check ends in "I think that
   worked?"
3. **One path, the shortest that works.** Alternatives, options and caveats go at the
   end or in a separate reference; a guide that branches three times in step two is a
   guide nobody finishes.
4. **Every step is an action with a visible result.** "Click Save — the row turns
   green." A step with no observable result is where a reader silently diverges from
   the guide and never recovers.
5. **Then actually follow it, from a clean state.** Fresh directory, signed out,
   nothing cached. This is the whole method: the errors you hit are exactly the ones
   your reader will, and they are invisible from where you were sitting.
6. **Record the failures you hit** as a short troubleshooting list, with the message
   they will actually see so it is searchable.
7. Deliver `outputs/<task-slug>.md`, and say in your summary that you followed it and
   from what state — an unfollowed guide is a draft, however careful.
