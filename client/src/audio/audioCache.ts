import { createStore, del, get, keys, set } from "idb-keyval";

import type { AudioThemeManifest } from "./types";

/** Dedicated IndexedDB store — isolates audio cache from card image cache. */
const audioStore = createStore("audio-cache", "audio-cache");

function audioCacheKey(
  themeId: string,
  assetType: "sfx" | "music",
  filename: string,
): string {
  return `audio:${themeId}:${assetType}:${filename}`;
}

function manifestKey(themeId: string): string {
  return `audio-manifest:${themeId}`;
}

export async function getCachedAudio(
  themeId: string,
  assetType: "sfx" | "music",
  filename: string,
): Promise<ArrayBuffer | null> {
  const data = await get<ArrayBuffer>(
    audioCacheKey(themeId, assetType, filename),
    audioStore,
  );
  return data ?? null;
}

export async function cacheAudio(
  themeId: string,
  assetType: "sfx" | "music",
  filename: string,
  data: ArrayBuffer,
): Promise<void> {
  await set(audioCacheKey(themeId, assetType, filename), data, audioStore);
}

export async function clearThemeCache(themeId: string): Promise<void> {
  const prefix = `audio:${themeId}:`;
  const allKeys = await keys<string>(audioStore);
  const toDelete = allKeys.filter((k) => k.startsWith(prefix));
  await Promise.all(toDelete.map((k) => del(k, audioStore)));
  await del(manifestKey(themeId), audioStore);
}

export async function fetchWithCache(
  url: string,
  themeId: string,
  assetType: "sfx" | "music",
  filename: string,
): Promise<ArrayBuffer> {
  const cached = await getCachedAudio(themeId, assetType, filename);
  if (cached) return cached;

  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to fetch audio: HTTP ${response.status} for ${url}`);
  }
  const data = await response.arrayBuffer();
  await cacheAudio(themeId, assetType, filename, data);
  return data;
}

export async function cacheThemeManifest(
  themeId: string,
  manifest: AudioThemeManifest,
): Promise<void> {
  await set(manifestKey(themeId), manifest, audioStore);
}

export async function getCachedManifest(
  themeId: string,
): Promise<AudioThemeManifest | null> {
  const manifest = await get<AudioThemeManifest>(
    manifestKey(themeId),
    audioStore,
  );
  return manifest ?? null;
}
