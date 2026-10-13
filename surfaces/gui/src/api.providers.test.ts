import { afterEach, expect, it, vi } from "vitest";
import { getProviders } from "./api";

// The provider list is mapped and searched wherever it is used, so an
// error body must come back as an empty list rather than as an object with no `.find`.

afterEach(() => {
  vi.unstubAllGlobals();
});

const answering = (status: number, body: unknown) =>
  vi.fn(async () => ({ ok: status < 400, status, json: async () => body }) as Response);

it("reads a 401 before the launch token lands as no providers, not a crash", async () => {
  vi.stubGlobal("fetch", answering(401, { error: "unauthorized" }));
  await expect(getProviders()).resolves.toEqual([]);
});

it("reads a body that isn't JSON at all as no providers", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => ({ ok: false, status: 502, json: async () => { throw new Error("not json"); } }) as unknown as Response));
  await expect(getProviders()).resolves.toEqual([]);
});

it("passes a real list through untouched", async () => {
  const list = [{ name: "ollama", configured: true }];
  vi.stubGlobal("fetch", answering(200, list));
  await expect(getProviders()).resolves.toEqual(list);
});
