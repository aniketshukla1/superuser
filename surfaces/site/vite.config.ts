import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The marketing site, separate from `surfaces/gui`. The app is a Tauri desktop
// shell that has to load from a `tauri://` origin; this is a plain static site
// that gets hosted. Sharing one build would mean one set of compromises serving
// neither — so they are siblings, and only the design tokens travel between them.
//
// 1422, because the app's dev server owns 1421 and both need to run at once when
// the site is showing a preview of the app UI.
export default defineConfig({
  plugins: [react()],
  // GitHub Pages serves the site from /superuser/; the workflow sets SITE_BASE for that.
  // Locally it stays at the root.
  base: process.env.SITE_BASE || "/",
  server: { port: 1422, strictPort: true },
});
