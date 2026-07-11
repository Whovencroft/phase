import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("idb-keyval", () => {
  const db = new Map<string, unknown>();
  return {
    createStore: vi.fn(() => ({})),
    get: vi.fn((key: string) => Promise.resolve(db.get(key) ?? undefined)),
    set: vi.fn((key: string, value: unknown) => {
      db.set(key, value);
      return Promise.resolve();
    }),
    del: vi.fn((key: string) => {
      db.delete(key);
      return Promise.resolve();
    }),
    keys: vi.fn(() => Promise.resolve([...db.keys()])),
    _db: db, // exposed for test cleanup
  };
});

// Get the mock's internal db reference for cleanup
import * as idbKeyval from "idb-keyval";
const getDb = () => (idbKeyval as unknown as { _db: Map<string, unknown> })._db;

import {
  cacheAudio,
  cacheThemeManifest,
  clearThemeCache,
  getCachedAudio,
  getCachedManifest,
} from "../audioCache";
import type { AudioThemeManifest } from "../types";

describe("audioCache", () => {
  beforeEach(() => {
    getDb().clear();
  });

  it("stores and retrieves audio data", async () => {
    const data = new ArrayBuffer(16);
    await cacheAudio("theme1", "sfx", "damage.mp3", data);
    const result = await getCachedAudio("theme1", "sfx", "damage.mp3");
    expect(result).toBe(data);
  });

  it("returns null for cache miss", async () => {
    const result = await getCachedAudio("theme1", "sfx", "missing.mp3");
    expect(result).toBeNull();
  });

  it("clears all entries for a theme", async () => {
    await cacheAudio("theme1", "sfx", "a.mp3", new ArrayBuffer(8));
    await cacheAudio("theme1", "music", "b.mp3", new ArrayBuffer(8));
    await cacheAudio("theme2", "sfx", "c.mp3", new ArrayBuffer(8));

    await clearThemeCache("theme1");

    expect(await getCachedAudio("theme1", "sfx", "a.mp3")).toBeNull();
    expect(await getCachedAudio("theme1", "music", "b.mp3")).toBeNull();
    // theme2 untouched
    expect(await getCachedAudio("theme2", "sfx", "c.mp3")).not.toBeNull();
  });

  it("stores and retrieves theme manifests", async () => {
    const manifest: AudioThemeManifest = {
      id: "test",
      name: "Test",
      version: 1,
      sfx: [],
      music: { battlefield: [{ id: "bf-1", url: "/bf.mp3" }] },
    };

    await cacheThemeManifest("test", manifest);
    const result = await getCachedManifest("test");
    expect(result).toEqual(manifest);
  });

  it("returns null for missing manifest", async () => {
    const result = await getCachedManifest("nonexistent");
    expect(result).toBeNull();
  });

  it("clearThemeCache also removes the manifest", async () => {
    const manifest: AudioThemeManifest = {
      id: "test",
      name: "Test",
      version: 1,
      sfx: [],
      music: { battlefield: [{ id: "bf-1", url: "/bf.mp3" }] },
    };

    await cacheThemeManifest("test", manifest);
    await cacheAudio("test", "sfx", "a.mp3", new ArrayBuffer(8));
    await clearThemeCache("test");

    expect(await getCachedManifest("test")).toBeNull();
    expect(await getCachedAudio("test", "sfx", "a.mp3")).toBeNull();
  });
});
