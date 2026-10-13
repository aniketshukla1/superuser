import { test, expect, boot, shot } from "./audit";

// The product's signature interaction — a tool asking permission — judged as UX,
// not just as behaviour. The regression suite already proves the card appears;
// these ask whether a keyboard or screen-reader user can actually act on it, and
// whether a long turn is narrated to anyone not watching the pixels.

/** The fixture's fake agent proposes a tool when a message contains "run a tool". */
async function askForATool(page: any) {
  const composer = page.locator("textarea").first();
  await composer.fill("please run a tool for me");
  await composer.press("Enter");
  await page.waitForTimeout(1600);
}

test("a permission request is announced to assistive tech", async ({ page }) => {
  await boot(page);
  await askForATool(page);
  await shot(page, "flow-approval-card");

  const live = await page.evaluate(() => {
    const regions = Array.from(document.querySelectorAll("[aria-live], [role=alert], [role=status]")) as HTMLElement[];
    return regions
      .filter((r) => r.getBoundingClientRect().height > 0)
      .map((r) => ({
        role: r.getAttribute("role"),
        live: r.getAttribute("aria-live"),
        text: (r.textContent || "").trim().slice(0, 80),
      }));
  });

  const approvalText = await page.locator("body").innerText();
  expect(approvalText, "the fixture agent should have proposed a tool").toMatch(/approve|allow|permission|run/i);

  const announces = live.some((r) => /approv|permission|allow|tool/i.test(r.text));
  expect(
    { liveRegions: live, announces },
    `the app is blocked waiting on the user, but nothing is in a live region — a screen-reader user is never told:\n${JSON.stringify(live, null, 2)}`,
  ).toMatchObject({ announces: true });
});

test("a permission request can be answered from the keyboard alone", async ({ page }) => {
  await boot(page);
  await askForATool(page);

  // Where does focus go when the app starts blocking on a decision?
  const focusOnArrival = await page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    return {
      tag: el?.tagName.toLowerCase() ?? "body",
      label: (el?.getAttribute("aria-label") || el?.textContent || "").trim().slice(0, 40),
      insideApproval: !!el?.closest("[class*=approval], [class*=appr]"),
    };
  });

  // Tab until an approve control is reached, and count how far it is.
  let steps = -1;
  for (let i = 0; i < 40; i++) {
    await page.keyboard.press("Tab");
    const hit = await page.evaluate(() =>
      /allow|approve|yes/i.test((document.activeElement?.textContent || "").trim()),
    );
    if (hit) {
      steps = i + 1;
      break;
    }
  }

  expect(
    { focusOnArrival, tabsToReachApprove: steps },
    `focus is not moved to the decision the app is blocked on; reaching Approve took ${steps} Tab presses`,
  ).toMatchObject({ focusOnArrival: { insideApproval: true } });
});

test("reduced motion is honoured", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await boot(page);
  await shot(page, "reduced-motion");
  const animating = await page.evaluate(() => {
    const out: any[] = [];
    for (const el of Array.from(document.querySelectorAll("*")) as HTMLElement[]) {
      const cs = getComputedStyle(el);
      if (cs.animationName !== "none" && parseFloat(cs.animationDuration) > 0.15) {
        out.push({
          cls: el.className?.toString().slice(0, 44) || el.tagName,
          animation: cs.animationName,
          duration: cs.animationDuration,
          iteration: cs.animationIterationCount,
        });
      }
    }
    return out.slice(0, 12);
  });
  const looping = animating.filter((a) => a.iteration === "infinite");
  expect(
    looping,
    `these keep animating forever under prefers-reduced-motion:\n${JSON.stringify(looping, null, 2)}`,
  ).toEqual([]);
});

test("a running turn is narrated, not just spun at", async ({ page }) => {
  await boot(page);
  const composer = page.locator("textarea").first();
  await composer.fill("hello");
  await composer.press("Enter");
  await page.waitForTimeout(500);
  const live = await page.evaluate(() =>
    (Array.from(document.querySelectorAll("[aria-live], [role=status], [role=alert]")) as HTMLElement[])
      .filter((r) => r.getBoundingClientRect().height > 0)
      .map((r) => ({ live: r.getAttribute("aria-live"), role: r.getAttribute("role"), text: (r.textContent || "").trim().slice(0, 60) })),
  );
  await shot(page, "flow-running-turn");
  expect(
    live.length,
    "nothing on the page reports turn progress to assistive tech while the agent works",
  ).toBeGreaterThan(0);
});
