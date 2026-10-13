---
name: design-review
description: Critique a screen against the system it belongs to, with the reasons stated
allowed-tools: grep, list_dir, read_file, write_file
---
Judge the screen against its own system, not against taste. "I would have done it
differently" is not a review; "this heading is the only one in caps" is.

1. **Read the system first** — tokens, shared constants, the nearest sibling screen.
   You cannot say something is inconsistent until you know what it is inconsistent
   with.
2. **Look for the specific ways drift shows**, which are almost never "it looks bad":
   - a value reached the wrong way (a raw size where a token exists, so it carries no
     line-height),
   - one element styled unlike its siblings (a heading in caps beside three in title
     case),
   - a shade that is not on the palette (an opacity variant nobody contrast-checked),
   - a spacing rhythm broken by one element,
   - the same idea named twice, which is how two copies drift apart.
3. **Check hierarchy by squinting.** If everything is the same weight, nothing is
   emphasised; if the description is bigger than the title, the page argues with
   itself. Say which element should recede.
4. **Check it with real content**: the longest name, the empty list, the error. Most
   designs are drawn with content that fits.
5. **Separate the two kinds of finding.** Deviations from the system are fixable now
   and should just be fixed. Judgement calls — this hierarchy, this density — belong
   to whoever owns the design, so put them as questions rather than corrections.
6. Deliver `outputs/design-review.md`: findings as element · what it does · what the
   system does · fix. Fix the deviations; list the judgement calls separately, so
   nobody has to guess which is which.
