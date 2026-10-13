---
name: frontend-a11y
description: Make a screen usable without a mouse or without sight, and prove which parts you checked
allowed-tools: card, grep, list_dir, read_file, write_file
---
Work one screen at a time and report honestly on coverage. An accessibility pass
whose scope is unstated reads as a clean bill of health for the whole app.

1. **Keyboard first, mouse unplugged.** Tab through the screen. Every interactive
   thing must be reachable, in an order that matches the visual one, with a focus
   ring you can actually see. Anything reachable that does nothing, or unreachable
   that does something, is a finding.
2. **Names, not shapes.** Every control needs an accessible name — an icon-only
   button with no label is a button that announces nothing. Check the name says what
   the control DOES, not what it looks like: "Delete assistant", not "trash".
3. **Live changes must be announced.** A card that appears, a save that fails, a
   turn that finishes — if it is only visual, a screen-reader user is told nothing.
   That is what a live region is for, and a blocking prompt should take focus.
4. **Contrast, measured not eyeballed.** Text against every surface it can land on,
   not just the darkest one — a dim grey flatters itself on the darkest background
   and fails on the lightest. 4.5:1 for body, 3:1 for large text and UI edges.
5. **Do not rely on colour alone** for state. A red border and no text is invisible
   to a large minority of people.
6. **Respect the OS**: reduced motion, increased contrast, and a font size the user
   chose.
7. Deliver `outputs/a11y-review.md`: findings as element · what a keyboard or
   screen-reader user experiences · the fix — and a Coverage note saying which
   screens you checked and which you did not. Fix what you can; list what needs a
   design decision.
