// A one-line channel for "that didn't work".
//
// Before this, a write that the server refused was invisible: Settings toggles set
// their local state and awaited the call without looking at the result, so the switch
// stayed in its new position while the backend kept the old value; the automations
// detail toggle reconciled on refresh, so the switch sprang back with no explanation
// (UX-REPORT §5). Both read to the user as "the app is broken" or, worse, as success.
//
// Deliberately tiny: a module-level list plus a subscription, so any component can
// report a failure without threading a prop through the tree, and one <Notices/> in
// App renders them in a `role="alert"` region.

export interface Notice {
  id: number;
  text: string;
  /** Optional detail from the server, shown small under the message. */
  detail?: string;
}

const listeners = new Set<(all: Notice[]) => void>();
let notices: Notice[] = [];
let nextId = 1;

const publish = () => listeners.forEach((fn) => fn(notices));

/** Show a failure. Auto-dismisses; the user can also close it. */
export function notifyError(text: string, detail?: string): number {
  const id = nextId++;
  notices = [...notices, { id, text, detail }];
  publish();
  setTimeout(() => dismissNotice(id), 8000);
  return id;
}

export function dismissNotice(id: number) {
  const next = notices.filter((n) => n.id !== id);
  if (next.length === notices.length) return;
  notices = next;
  publish();
}

export function subscribeNotices(fn: (all: Notice[]) => void): () => void {
  listeners.add(fn);
  fn(notices);
  return () => listeners.delete(fn);
}

/** `if (!(await reportIfFailed(setOnboarded(false), "Couldn't save that setting.")))` —
 *  true when the write landed, false when it didn't (and the user has been told).
 *
 *  Anything other than an explicit `ok: true` counts as a failure. Every endpoint used
 *  this way answers `{ok: boolean}`, and a 500 whose body is `{"error": "..."}` has no
 *  `ok` at all — treating a *missing* ok as success is precisely how these writes came
 *  to look like they had worked. */
export async function reportIfFailed(
  call: Promise<{ ok?: boolean; error?: string } | null | undefined>,
  message: string,
): Promise<boolean> {
  try {
    const res = await call;
    if (res?.ok === true) return true;
    notifyError(message, res?.error);
    return false;
  } catch (e) {
    notifyError(message, e instanceof Error ? e.message : undefined);
    return false;
  }
}
