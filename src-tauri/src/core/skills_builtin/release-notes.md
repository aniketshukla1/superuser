---
name: release-notes
description: Write release notes from the actual commits, in terms of what changed for the reader
allowed-tools: write_file
---
Notes written from memory list what the team found interesting. Notes written from
the log list what shipped — which is a different set, and the one users need.

1. **Read the range**: `git log <last-tag>..HEAD`. If there is no tag, say which
   range you used, because notes over an unstated range are unverifiable.
2. **Group by what the reader does**, not by subsystem: what is new, what changed in
   behaviour they will notice, what is fixed, what they must act on. "Refactored the
   permission engine" belongs in none of those and should be dropped.
3. **Write each line as the effect, not the change.** "The Board tab now appears for
   any project" over "changed panelTabs to take hasBoard". If you cannot phrase a
   commit as an effect, it is probably internal and does not belong here.
4. **Lead with breaking changes and required actions**, even if there is one and
   forty features. A migration someone missed is worse than a feature they did not
   notice.
5. **Say what is still broken** if a known issue survived the release. Notes that
   only ever add are notes people stop reading during an incident.
6. **Name nothing internal**: no file paths, no ticket ids the reader cannot open, no
   internal service names.
7. Deliver `outputs/release-notes.md`, newest version at the top, dated, with the
   commit range recorded at the bottom so the next person can pick up where you left.
