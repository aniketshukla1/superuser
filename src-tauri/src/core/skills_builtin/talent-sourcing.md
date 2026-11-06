---
name: talent-sourcing
description: Find candidates who meet a role's must-haves in public professional work, leave out anyone already in the ATS, and draft outreach in the person's voice
allowed-tools: web_search, web_fetch, candidates_check, candidates_shortlist, write_file, recall_memory, todo_write
---
Find people whose public work shows they meet the bar, prove each match, and draft outreach the
person sends themselves. Contact no one.

1. **Set the bar.** From the role: three to five must-haves, each checkable from public work, and
   the nice-to-haves. "Senior" isn't checkable; "has maintained a Kubernetes operator" is. Ask when
   a must-have can't be checked.
2. **Search where the work is.** `web_search` for the work itself — repositories, maintainers,
   conference speakers, papers, release notes. GitHub's public API answers without a login:
   `https://api.github.com/search/users?q=language:rust+location:berlin`, and a person at
   `https://api.github.com/users/<login>`. Keep the link you found each person by.
3. **Evidence for every must-have.** A link and what it shows: "maintains kube-rs — 300 commits
   since 2023". A must-have without evidence is a miss, not a maybe.
4. **Check.** `candidates_check` with everyone who meets the bar. Leave out whoever it says to.
   A same-name match stays out unless the evidence shows it's someone else.
5. **Rank** by the evidence, nice-to-haves breaking ties. Stop at the number asked for — 20 when
   none was — and say so when fewer meet the bar.
6. **Shortlist** them with `candidates_shortlist` for the role, so the next search leaves them out.
7. **Draft outreach** for each in the person's voice — the sample `candidates_check` gives: one
   specific thing from their work, why this role, one question, no flattery, under 120 words.
8. **Write `outputs/shortlist.md`:** the bar; each candidate with their links, the evidence for
   each must-have and the draft; then who was left out and why — in the ATS, shortlisted before,
   or a must-have unmet.
