---
name: api-reference
description: Document an interface from its implementation, so the docs cannot quietly drift
allowed-tools: grep, list_dir, read_file, write_file
---
Reference docs rot because they are written beside the code rather than from it. Read
the handler, not the ticket.

1. **Enumerate the surface from the source**: every route, its methods, and where each
   is registered. A route someone forgot to document and a route someone forgot to
   register look identical from the outside — reading the registration finds both.
2. **For each entry, from the code**: path, method, auth, request shape with types and
   which fields are required, response shape, and every status it can return.
3. **Document the errors as carefully as the successes.** The success shape is
   guessable; what a malformed body returns is not, and that is what someone reaches
   the docs for at 2am.
4. **Give one real example per entry** — a request and its actual response, copied
   from a real call, not invented. Invented examples are where the drift starts, and
   they are the part readers paste.
5. **Say what is stable and what is not.** A reader cannot tell an intentional
   contract from an accident of the current implementation, and will depend on both.
6. **Note anything the code does that the name does not suggest** — a side effect, a
   rate limit, an idempotency rule, a field that is ignored.
7. Deliver `outputs/api-reference.md`, and list anything you found that looks wrong
   rather than documenting it as intended: docs written from a bug make the bug
   permanent.
