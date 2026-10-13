---
name: ui-component
description: Build a UI component that belongs to the design system rather than beside it
allowed-tools: card, grep, list_dir, read_file, run_shell
---
The failure this exists to prevent: a component that looks right in isolation and
wrong beside everything else, because each of its numbers was a reasonable local
decision made with nowhere to look the answer up.

1. **Find the system before writing any CSS.** Locate the token definitions — type
   scale, spacing, radii, colour — and the shared class constants. If a value you
   need has a name, use the name; a raw `13px` is the same size and a different
   thing, because the token carries its line-height and the raw number does not.
2. **Find the nearest existing component and match it.** Its section heading, its
   card, its row rhythm. Two components doing the same job differently is the drift
   that a design system exists to stop, and copying a constant into your file is how
   it starts.
3. **Build the states, not just the happy one**: loading, empty, error, and the one
   where the data is three times longer than the mock. An empty state that explains
   what would fill it teaches the feature; a blank panel hides it.
4. **Keyboard and screen reader as you go, not after**: reachable by Tab in a sane
   order, a visible focus ring, a label on every control, and an announcement for
   anything that changes without a click.
5. **Check both themes and a narrow width.** Most colour mistakes only appear in one
   theme, and most layout mistakes only at the width you did not try.
6. **Test what the component promises**, not that it renders: the empty state says
   what it should, the failed save puts the control back, the disabled reason is
   shown rather than implied.
7. Deliver the component and its `<Component>.test.tsx` beside it, and say in your
   summary which tokens you used and anything you had to invent — an invented value is the next drift, and
   naming it is how it gets a token instead.
