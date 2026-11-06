import { defineConfig, devices } from "@playwright/test";

// Local-only: the UX screenshot audit (e2e-ux) for the design-pass worktree, on its own port so
// it can never reuse another checkout's dev server. Screenshots land in ux-shots/ (gitignored).
// Not for committing.
const PORT = 5298;

export default defineConfig({
  testDir: "./e2e-ux",
  fullyParallel: true,
  retries: 0,
  reporter: [["line"]],
  timeout: 180_000,
  use: { baseURL: `http://localhost:${PORT}`, trace: "off" },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], channel: "chrome" } }],
  webServer: {
    command: `npm run dev -- --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
