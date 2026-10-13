import { defineConfig } from "@playwright/test";

// REAL-APP config. No browser, no web server — these specs drive the running desktop app
// through the test bridge (src/testbridge.ts ↔ e2e-app/driver.ts).
//
//   1. npm run app          # start the desktop app (dev build; the bridge is dev-only)
//   2. npx playwright test -c playwright.app.config.ts
//
// Serial on purpose: there is one app and one window, and two specs resizing it at once
// would each see the other's changes. This is a handful of probes about the window
// manager and the OS chrome, not a suite to parallelise.
export default defineConfig({
  testDir: "./e2e-app",
  testMatch: "*.spec.ts",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [["list"]],
  timeout: 60_000,
});
