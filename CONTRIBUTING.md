# Contributing to Superuser

Thanks for helping. Bug reports and ideas go in [issues](https://github.com/aniketshukla1/superuser/issues). For a larger change, open an issue first so we can agree on the shape before you write it.

## Build and run

Follow the [Quickstart](README.md#-quickstart). In short, on macOS 13 or later with Rust, Node.js 20+, CMake and the Xcode command line tools:

```bash
npm install
npm --prefix surfaces/gui ci
npm run app
```

`npm run app` starts the interface's dev server and the Rust core together, with hot reload for the interface.

## Where things live

```
src-tauri/                  the Rust core (a Tauri 2 app)
  src/core/                 agent loop, gateway, boundaries, gates, memory, egress, scheduler, audit
  src/core/personas_builtin/  the built-in assistants, one markdown file each
  src/core/skills_builtin/    the built-in skills, one markdown file each
  src/integrations/         connectors (Gmail, Slack, GitHub, Prometheus…)
  src/core/api_server.rs    the local HTTP and WebSocket API the interface talks to
surfaces/gui/               the desktop interface (React + Vite); also runs in a browser for development
  e2e/                      Playwright specs over a mock backend (e2e/fixtures.ts)
  e2e-demo/                 the README's screenshots and demo GIF, over the same mock
surfaces/site/              the landing page
scripts/                    build helpers and the README asset generator
```

## Tests

```bash
npm run test:gui                                          # interface unit tests (vitest)
(cd surfaces/gui && npx tsc --noEmit -p tsconfig.json)    # interface types
(cd src-tauri && CARGO_INCREMENTAL=0 cargo test --lib)    # Rust core
```

End-to-end specs drive the real interface in Chrome against a mock backend, so they need no Rust build:

```bash
cd surfaces/gui
npx playwright install chromium    # once
npx playwright test
```

Some end-to-end specs still assert older copy and layouts. Bringing one up to date is a good first contribution; CI runs the unit tests and the Rust suite, not these.

The benchmark behind the README's local-model table needs a running app and Ollama. Its header comment, in [`src-tauri/src/harness_eval.rs`](src-tauri/src/harness_eval.rs), explains how to run it without touching your own data.

## Adding an assistant or a skill

A built-in assistant is a markdown file in `src-tauri/src/core/personas_builtin/`: a YAML header (name, tagline, tools, skills, default approval mode) and a body that is its instructions. Copy the closest one and edit it. Skills work the same way in `src-tauri/src/core/skills_builtin/`. People can also create their own in the app, without a build.

## House rules

- **Every outbound request goes through `core::egress`.** A test fails if code builds an HTTP client anywhere else. This is what keeps the Network page and local-only mode honest.
- **Every tool call goes through `core::gateway`**, so boundaries and the audit trail apply to it. A new tool gets that for free by being dispatched the usual way.
- **Match the code around you.** The interface has no formatter: don't run Prettier over existing files, because it rewraps them.
- **Comments say why.** What the code does is in the code. Why it does that, and what it must not do, belongs in the comment.
- **Interface copy is plain.** Say what happened and what to do next, in words a non-engineer can follow. No jokes in error messages.
- **Tests come with the change.** A bug fix gets a test that fails without it.

## Regenerating the README images

The screenshots and the demo GIF are the real interface over the mock backend, with invented data from `surfaces/gui/e2e-demo/`:

```bash
cd surfaces/gui
node node_modules/@playwright/test/cli.js test -c playwright.demo.config.ts   # raw captures
python3 ../../scripts/readme-assets.py                                        # sized images and the GIF
```

`python3 scripts/readme-assets.py --extract` refreshes the assistants and boundary presets the demo shows, straight from the Rust source.

## Contribution terms

Superuser is licensed under the [PolyForm Strict License 1.0.0](LICENSE), which does not allow changes. As an additional permission, you may change the code solely to prepare a contribution to this repository.

By submitting a contribution (an issue, a patch or a pull request), you confirm that you wrote it or have the right to submit it, and you grant Aniket Shukla a perpetual, worldwide, non-exclusive, royalty-free and irrevocable licence to use, reproduce, modify, sublicense and distribute it under any terms, including commercial ones. You keep the copyright in your contribution.
