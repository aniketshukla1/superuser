

// Second UX pass. The first (a11y / layout / resilience / stress / tour / whiteScreen)
// asked whether each surface renders and is reachable. These ask what happens
// *after* the first click: does a failure reach the user, does a decision survive a
// reload, does the keyboard get out of a modal, does the shell hold at the window
// size the product itself allows.
//
// Same rules as the rest of e2e-ux: every probe reports the facts it gathered in the
// failure message, so a red test names a defect instead of a line number.

// ─────────────────────────────────────────────────────────────────────────────
// helpers
// ─────────────────────────────────────────────────────────────────────────────

/** Rewrite every JSON response in place, keeping the envelope. Used to make an
 *  "empty account" that is shaped exactly like the real backend's — emptying at
 *  the network layer with `[]` tests the client's parser, not its empty state. */
export async function emptyEveryList(page: any) {
  await page.addInitScript(() => {
    const strip = (v: any): any => {
      if (Array.isArray(v)) return [];
      if (v && typeof v === "object") {
        const out: any = {};
        for (const [k, x] of Object.entries(v)) out[k] = strip(x);
        return out;
      }
      return v;
    };
    const orig = window.fetch;
    window.fetch = async (input: any, init?: any) => {
      const res = await orig(input, init);
      const url = typeof input === "string" ? input : input?.url || "";
      if (!/\/v1\//.test(url)) return res;
      // Settings and health decide whether the app boots at all; leave them alone.
      if (/\/v1\/(health|settings)/.test(url)) return res;
      const body = await res.clone().json().catch(() => null);
      if (body === null) return res;
      return new Response(JSON.stringify(strip(body)), {
        status: res.status,
        headers: { "content-type": "application/json" },
      });
    };
  });
}

export const visibleText = async (page: any) =>
  (await page.locator("body").innerText()).replace(/\s+/g, " ").trim();

// ─────────────────────────────────────────────────────────────────────────────
// 3. Modality
// ─────────────────────────────────────────────────────────────────────────────

/** Open the palette the only way the product allows — by clicking Search. */
export async function openPalette(page: any) {
  await page.getByRole("button", { name: /search/i }).first().click();
  await page.waitForTimeout(350);
}
