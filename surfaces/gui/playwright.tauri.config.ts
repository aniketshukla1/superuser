import { defineConfig, devices } from "@playwright/test";

// TAURI AUDIT config — the surface `playwright.ux.config.ts` structurally cannot reach.
//
// Two differences from the browser build, and this config exists for both:
//
//  1. ENGINE. The desktop shell is a WKWebView, i.e. WebKit — not Chromium. Every probe
//     in e2e-ux/ has only ever run in Chromium, so a CSS or DOM feature that Chromium
//     supports and WebKit does not would pass every test and ship broken. Playwright's
//     WebKit build is not literally WKWebView, but it is the same engine family and the
//     only automatable proxy on this machine (tauri-driver has no macOS support).
//
//  2. CHROME. On macOS the app draws its own title bar over the traffic lights
//     (`titleBarStyle: "Overlay"`, `.app.tauri-overlay`). None of that CSS is reachable
//     in a browser except through the `?overlay=1` dev flag (App.tsx:1303) — which is
//     how the shipped build came to render its wordmark as "superuser bet" with no test
//     failing anywhere.
//
//   npx playwright test -c playwright.tauri.config.ts
//
// The `webkit` project re-runs the FULL existing suite on the other engine; `overlay`
// runs the desktop-chrome probes in e2e-tauri/tauri/.
const PORT = 5199;

export default defineConfig({
  testDir: ".",
  fullyParallel: true,
  retries: 0,
  reporter: [["list"]],
  timeout: 90_000,
  use: {
    baseURL: `http://localhost:${PORT}`,
    trace: "off",
  },
  projects: [
    {
      name: "webkit",
      testDir: "./e2e-ux",
      testIgnore: "tauri.spec.ts",
      use: { ...devices["Desktop Safari"] },
    },
    {
      name: "overlay",
      testDir: "./e2e-tauri",
      use: { ...devices["Desktop Safari"] },
    },
  ],
  webServer: {
    command: `npm run dev -- --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: true,
    timeout: 120_000,
  },
});
