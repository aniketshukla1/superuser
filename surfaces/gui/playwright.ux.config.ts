import { defineConfig, devices } from "@playwright/test";

// UX AUDIT config — a separate, exploratory pass over the same hermetic mock the
// regression suite uses (`e2e/fixtures.ts`). Kept out of `playwright.config.ts`
// (testDir ./e2e) so `npm run e2e` stays a clean pass/fail gate: these specs are
// deliberately noisy — they screenshot every surface and assert design/a11y
// properties that the product may knowingly not meet yet.
//
// Run it with @playwright/test's OWN cli, not `npx playwright`. Both `playwright` and
// `@playwright/test` ship a bin called `playwright`, pnpm linked the former into
// node_modules/.bin, and that CLI has a different test registry — so every spec file
// fails to collect with "Playwright Test did not expect test() to be called here",
// which reads like a broken suite rather than a wrong binary. That is why the brand-row
// guard below sat un-run while the bug it guards against shipped twice.
//
//   node node_modules/@playwright/test/cli.js test -c playwright.ux.config.ts
//
// `channel: "chrome"` uses the Chrome already on the machine rather than Playwright's
// own download, so the suite runs on a fresh checkout without a 150MB fetch.
//
// Screenshots land in `ux-shots/`.
const PORT = 5199;

export default defineConfig({
  testDir: "./e2e-ux",
  fullyParallel: true,
  retries: 0,
  reporter: [["list"]],
  timeout: 90_000,
  use: {
    baseURL: `http://localhost:${PORT}`,
    trace: "off",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], channel: "chrome" } }],
  webServer: {
    command: `npm run dev -- --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: true,
    timeout: 120_000,
  },
});
