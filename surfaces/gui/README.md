# Superuser interface

The desktop interface for Superuser: React, TypeScript and Vite. Inside the app, Tauri loads it and the Rust core serves it an HTTP and WebSocket API on `127.0.0.1` (`src-tauri/src/core/api_server.rs`).

## Run it

From the repository root, `npm run app` starts this dev server and the Rust core together. That is the normal way to work on it.

To run only the interface in a browser against an app that is already running:

```bash
npm install
npm run dev        # → http://localhost:1421
```

It finds the API at `http://127.0.0.1:8765` (override with `VITE_COWORKER_HTTP` and `VITE_COWORKER_WS`) and reads that launch's token from the app's data folder.

## Test it

```bash
npm test                          # unit tests (vitest, jsdom)
npx tsc --noEmit -p tsconfig.json # types
npx playwright test               # end-to-end, in Chrome, over the mock backend in e2e/fixtures.ts
```

The end-to-end specs need no Rust build: `e2e/fixtures.ts` answers every API call the interface makes. `e2e-demo/` uses the same mock to produce the README's screenshots; see [CONTRIBUTING.md](../../CONTRIBUTING.md#regenerating-the-readme-images).
