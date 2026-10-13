#!/usr/bin/env node
// Drop CMake build dirs whose cached macOS deployment target predates the one this
// project needs, so the next build reconfigures instead of reusing it.
//
// Why this has to exist. whisper.cpp's ggml uses std::filesystem::path, which libc++
// marks unavailable before macOS 10.15, so the build needs a floor of 11.0.
// .cargo/config.toml sets MACOSX_DEPLOYMENT_TARGET for that reason. CMake seeds
// CMAKE_OSX_DEPLOYMENT_TARGET from it when it CONFIGURES a build directory, and then
// caches it — so a directory configured while the variable was missing keeps 10.13
// afterwards, no matter how correct the environment becomes.
//
// That is a transitional state, but a real one: any tree built before the config moved
// to the repo root has such a directory, and the build dies in whisper-rs-sys with
// twenty "'path' is unavailable" errors that look like they arrived with whatever you
// changed last. This makes it self-healing rather than a paragraph in a README.
//
// It is NOT the fix. The fix is the config being somewhere cargo actually reads it.
//
// Scope is deliberately narrow: only CMakeCache.txt files under src-tauri/target (cargo
// build output, regenerable by definition), only when the cached value is BELOW the
// floor, and it removes the generated build directory rather than the cache file alone —
// a cache file removed on its own leaves CMake confused about a half-configured tree.
import { readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { join } from "node:path";

const FLOOR = 11.0;
const ROOT = new URL("../src-tauri/target", import.meta.url).pathname;

const caches = [];
(function walk(dir, depth) {
  if (depth > 6) return;
  let entries;
  try { entries = readdirSync(dir, { withFileTypes: true }); } catch { return; }
  for (const e of entries) {
    const p = join(dir, e.name);
    if (e.isDirectory()) walk(p, depth + 1);
    else if (e.name === "CMakeCache.txt") caches.push(p);
  }
})(ROOT, 0);

let dropped = 0;
for (const cache of caches) {
  let cached;
  try {
    cached = readFileSync(cache, "utf8")
      .split("\n")
      .find((l) => l.startsWith("CMAKE_OSX_DEPLOYMENT_TARGET:"))
      ?.split("=")[1]
      ?.trim();
  } catch { continue; }
  if (!cached || !(parseFloat(cached) < FLOOR)) continue;
  const buildDir = cache.slice(0, -"/CMakeCache.txt".length);
  // Refuse to touch anything outside cargo's own output, whatever the walk turned up.
  if (!buildDir.startsWith(ROOT + "/") || !statSync(buildDir).isDirectory()) continue;
  console.log(`preflight: ${buildDir} is configured for macOS ${cached}, below the ${FLOOR} floor — removing so it reconfigures`);
  rmSync(buildDir, { recursive: true, force: true });
  dropped++;
}
if (dropped === 0) console.log(`preflight: ${caches.length} CMake build dir(s), all at or above macOS ${FLOOR}`);
