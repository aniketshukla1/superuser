---
name: api-design
description: Design or change an HTTP endpoint so its callers cannot be broken quietly
allowed-tools: grep, list_dir, read_file, run_shell, write_file
---
Design the interface before writing the handler.

1. **Find every caller first.** Search the whole repo, including other languages —
   a typed client in another project is still a caller, and a contract that spans two
   languages breaks silently: the call compiles, the response is missing a field, and
   nothing errors until someone reads a screen.
2. **Name the resource, not the action.** `POST /orders/:id/cancel` over
   `POST /cancelOrder`. Verbs live in the method.
3. **Decide the failure shapes before the success shape.** What does a missing id
   return? A malformed body? A valid request the state forbids? An endpoint whose
   errors were designed last returns 200 with an `ok: false` nobody checks.
4. **Answer with the whole resource after a write.** A client that has to guess what
   its own write produced will guess wrong, and optimistic UI is where that shows.
5. **Every field the client's type declares must be sent.** A required field the
   server omits is not a gap — it is a crash in whichever language does not tolerate
   `undefined`.
6. **Additive first.** New optional field: safe. Renamed field, narrowed type,
   removed value from an enum: a break, and it needs a deprecation window or a
   version. Say which you are doing.
7. **Write it down in `outputs/api-design.md`**: path, method, request, response,
   every error with its status, and the callers you found. Then implement it, and
   add a test that the response carries the fields the client reads.
