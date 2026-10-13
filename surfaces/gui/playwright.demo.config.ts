import { defineConfig, devices } from "@playwright/test";

// README assets — screenshots and the demo GIF's frames — taken from the real UI over a
// hermetic mock (e2e/fixtures.ts plus the demo data in e2e-demo/), so nothing in them is
// anybody's real mail, chats or accounts. Not a test suite: run it to regenerate the images.
//
//   node node_modules/@playwright/test/cli.js test -c playwright.demo.config.ts
//
// Uses the Chrome already installed (`channel: "chrome"`), like the UX audit config.
const PORT = 5199;

export default defineConfig({
  testDir: "./e2e-demo",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [["list"]],
  timeout: 120_000,
  use: {
    baseURL: `http://localhost:${PORT}`,
    trace: "off",
  },
  projects: [{ name: "chrome", use: { ...devices["Desktop Chrome"], channel: "chrome", deviceScaleFactor: 2 } }],
  webServer: {
    command: `npm run dev -- --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: true,
    timeout: 120_000,
  },
});
