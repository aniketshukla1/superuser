import {
  test,
  expect,
  boot,
  go,
  shot,
  watchErrors,
  SURFACES,
  VIEWPORTS,
  horizontalOverflow,
  structure,
} from "./audit";

// A screenshot + health sweep of every surface, at every window size the desktop
// shell can be dragged to, in both themes. Failures here are reported as data
// (attached to the run) rather than a bare assertion, so one bad surface doesn't
// hide the other six.

for (const vp of VIEWPORTS) {
  test(`tour @ ${vp.name} (${vp.width}x${vp.height})`, async ({ page }, testInfo) => {
    const errors = watchErrors(page);
    await page.setViewportSize({ width: vp.width, height: vp.height });
    await boot(page);

    const report: any[] = [];
    for (const s of SURFACES) {
      await go(page, s.hash);
      await shot(page, `${vp.name}-${s.name}`);
      const overflow = await horizontalOverflow(page);
      report.push({ surface: s.name, overflow });
    }

    await testInfo.attach("tour", { body: JSON.stringify({ report, errors }, null, 2), contentType: "application/json" });

    // A desktop shell must never scroll sideways at any size it allows.
    const sideways = report.filter((r) => r.overflow.doc > r.overflow.view + 1);
    expect(sideways, `horizontal overflow: ${JSON.stringify(sideways, null, 2)}`).toEqual([]);
    expect(errors, `console/page errors during the tour:\n${errors.join("\n")}`).toEqual([]);
  });
}

test("dark theme renders every surface", async ({ page }) => {
  await page.setViewportSize({ width: 1512, height: 945 });
  await page.addInitScript(() => localStorage.setItem("superuser.theme", "dark"));
  await boot(page);
  expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe("dark");
  for (const s of SURFACES) {
    await go(page, s.hash);
    await shot(page, `dark-${s.name}`);
  }
});

test("document structure: landmarks, headings, ids, lang", async ({ page }, testInfo) => {
  await boot(page);
  const found: any[] = [];
  for (const s of SURFACES) {
    await go(page, s.hash);
    found.push({ surface: s.name, ...(await structure(page)) });
  }
  await testInfo.attach("structure", { body: JSON.stringify(found, null, 2), contentType: "application/json" });

  // Collected, not short-circuited: one missing landmark must not hide the rest.
  const issues = {
    duplicateIds: found.filter((f) => f.duplicateIds.length).map((f) => ({ surface: f.surface, ids: f.duplicateIds })),
    noMain: found.filter((f) => f.landmarks.main === 0).map((f) => f.surface),
    noH1: found.filter((f) => f.landmarks.h1 === 0).map((f) => f.surface),
    manyH1: found.filter((f) => f.landmarks.h1 > 1).map((f) => ({ surface: f.surface, n: f.landmarks.h1 })),
    headingSkips: found
      .map((f) => {
        const bad: string[] = [];
        let prev = 0;
        for (const h of f.headings) {
          if (prev && h.level > prev + 1) bad.push(`h${prev} → h${h.level} ("${h.text}")`);
          prev = h.level;
        }
        return bad.length ? { surface: f.surface, bad } : null;
      })
      .filter(Boolean),
    imgsNoAlt: found.filter((f) => f.imgsNoAlt.length).map((f) => ({ surface: f.surface, imgs: f.imgsNoAlt })),
    headingsPerSurface: found.map((f) => ({ surface: f.surface, headings: f.headings })),
  };
  await testInfo.attach("issues", { body: JSON.stringify(issues, null, 2), contentType: "application/json" });

  expect(
    { ...issues, headingsPerSurface: undefined },
    `document-structure problems:\n${JSON.stringify(issues, null, 2)}`,
  ).toEqual({
    duplicateIds: [],
    noMain: [],
    noH1: [],
    manyH1: [],
    headingSkips: [],
    imgsNoAlt: [],
    headingsPerSurface: undefined,
  });
});

test("unknown routes do not strand the user", async ({ page }, testInfo) => {
  const errors = watchErrors(page);
  await boot(page);
  const probes = ["#/nope", "#/persona/does-not-exist", "#/chat/s-missing", "#/settings?tab=bogus", "#/brain?tab=bogus"];
  const seen: any[] = [];
  for (const hash of probes) {
    await go(page, hash);
    await page.waitForTimeout(400);
    seen.push({
      hash,
      landedOn: page.url().split("#")[1] || "",
      // "Stranded" = the app painted nothing readable.
      visibleText: (await page.locator("body").innerText()).replace(/\s+/g, " ").trim().slice(0, 120),
    });
    await shot(page, `route-${hash.replace(/[^a-z0-9]/gi, "_")}`);
  }
  await testInfo.attach("routes", { body: JSON.stringify({ seen, errors }, null, 2), contentType: "application/json" });
  const blank = seen.filter((s) => s.visibleText.length < 20);
  expect(blank, `routes that painted (almost) nothing: ${JSON.stringify(blank, null, 2)}`).toEqual([]);
  expect(errors, errors.join("\n")).toEqual([]);
});
