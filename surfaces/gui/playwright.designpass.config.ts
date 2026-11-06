import { defineConfig, devices } from "@playwright/test";
import base from "./playwright.config";

// Local-only runner for the design pass worktree: the hermetic suite, on its own port (so it
// can never reuse another checkout's dev server on 5199) and on the Chrome already installed
// (Playwright's bundled Chromium is not downloaded on this machine). Not for committing.
const PORT = 5299;

export default defineConfig({
  ...base,
  reporter: [["line"]],
  use: { ...base.use, baseURL: `http://localhost:${PORT}` },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], channel: "chrome" } }],
  webServer: {
    command: `npm run dev -- --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
