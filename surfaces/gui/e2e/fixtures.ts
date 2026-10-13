// The e2e fixtures: a `test` whose page has the API mocked before navigation. The mock lives
// in `fixtures/`: `mockApi.ts` wires it, `routes/` answers /v1 by area, `fakeAgent.ts` scripts
// the session socket, and `data.ts` / `catalog.ts` hold the payloads.
import { test as base, expect } from "@playwright/test";
import { mockApi } from "./fixtures/mockApi";

export { sendAppEvent } from "./fixtures/appEvents";
export { CLOUD_STATE } from "./fixtures/catalog";
export { mockApi };

// A `test` whose page has the API mocked before navigation.
export const test = base.extend({
  page: async ({ page }, use) => {
    await mockApi(page);
    // The launch film is the app's own "intro off" preference here, not a test switch:
    // every spec is a first launch in a fresh profile, and twelve seconds of film in
    // front of each one would test nothing but patience. intro.spec.ts turns it back on.
    await page.addInitScript(() => {
      try {
        localStorage.setItem("superuser.intro", "off");
      } catch {
        /* storage blocked: the reel's short cut plays, and Skip still works */
      }
    });
    await use(page);
  },
});

export { expect };
