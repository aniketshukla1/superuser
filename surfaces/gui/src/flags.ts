// Feature flags for launch. Each is read when it is used, not when the module loads, so a
// test or a running build can switch one through localStorage without a reload: "1" turns
// it on, "0" turns it off, and anything else leaves the shipped default.

function readFlag(name: string, shipped: boolean): boolean {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(name);
  } catch {
    // Storage unavailable (a torn-down jsdom, private browsing): use the default.
  }
  return stored === "1" ? true : stored === "0" ? false : shipped;
}

/** Assistant management: the Settings tab and the "Manage personas…" entry. On by default,
 *  because assistants are how this app organises work: the sidebar groups sessions under
 *  the assistant that ran them instead of showing one flat list of prompts. Tests switch
 *  it with `superuser.flag.personas`. */
export const showPersonas = () => readFlag("superuser.flag.personas", true);
