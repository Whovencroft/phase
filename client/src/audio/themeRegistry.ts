import { getCachedManifest } from "./audioCache";
import { PLANESWALKER_THEME } from "./planeswalkerTheme";
import type {
  AudioContextName,
  AudioThemeManifest,
  ResolvedTheme,
  ThemeTrack,
} from "./types";

// ---------------------------------------------------------------------------
// Built-in themes registry
// ---------------------------------------------------------------------------

/** Default theme used as fallback when a requested theme cannot be found. */
export const DEFAULT_THEME = PLANESWALKER_THEME;

/** All built-in themes keyed by ID. Used by findManifest and the theme selector UI. */
export const BUILT_IN_THEMES: Record<string, AudioThemeManifest> = {
  planeswalker: PLANESWALKER_THEME,
};

// ---------------------------------------------------------------------------
// Context inheritance
// ---------------------------------------------------------------------------

/** Parent context for inheritance: deck_builder→menu, lobby→menu, etc. */
export const CONTEXT_PARENTS: Record<AudioContextName, AudioContextName | null> =
  {
    menu: null,
    deck_builder: "menu",
    lobby: "menu",
    battlefield: null,
    victory: "battlefield",
    defeat: "battlefield",
  };

const ALL_CONTEXTS: AudioContextName[] = [
  "menu",
  "deck_builder",
  "lobby",
  "battlefield",
  "victory",
  "defeat",
];

// ---------------------------------------------------------------------------
// Theme resolution
// ---------------------------------------------------------------------------

function resolveUrl(url: string, baseUrl?: string): string {
  if (!baseUrl || url.startsWith("http://") || url.startsWith("https://") || url.startsWith("/")) {
    return url;
  }
  const base = baseUrl.endsWith("/") ? baseUrl : `${baseUrl}/`;
  return `${base}${url}`;
}

/**
 * Resolve a manifest into a runtime-ready theme.
 *
 * - Prepends `baseUrl` to relative URLs
 * - Eagerly resolves context inheritance (copies parent tracks into empty child contexts)
 * - Builds the sfxMap lookup
 */
export function resolveTheme(manifest: AudioThemeManifest): ResolvedTheme {
  const { baseUrl } = manifest;

  // Build sfxMap with resolved URLs
  const sfxMap: Record<string, string> = {};
  for (const entry of manifest.sfx) {
    sfxMap[entry.eventType] = resolveUrl(entry.url, baseUrl);
  }

  // Resolve URLs in music tracks
  const rawMusic: Record<AudioContextName, ThemeTrack[]> = {
    menu: [],
    deck_builder: [],
    lobby: [],
    battlefield: [],
    victory: [],
    defeat: [],
  };

  for (const ctx of ALL_CONTEXTS) {
    const tracks = manifest.music[ctx];
    if (tracks && tracks.length > 0) {
      rawMusic[ctx] = tracks.map((t) => ({
        ...t,
        url: resolveUrl(t.url, baseUrl),
        phase: t.phase ?? "any",
      }));
    }
  }

  // Eager inheritance: walk parent chain for empty contexts
  const musicByContext = { ...rawMusic };
  for (const ctx of ALL_CONTEXTS) {
    if (musicByContext[ctx].length > 0) continue;
    let parent = CONTEXT_PARENTS[ctx];
    while (parent) {
      if (musicByContext[parent].length > 0) {
        musicByContext[ctx] = musicByContext[parent];
        break;
      }
      parent = CONTEXT_PARENTS[parent];
    }
  }

  return { manifest, sfxMap, musicByContext };
}

// ---------------------------------------------------------------------------
// Theme manifest validation (system boundary)
// ---------------------------------------------------------------------------

export function validateThemeManifest(
  json: unknown,
): AudioThemeManifest | Error {
  if (typeof json !== "object" || json === null) {
    return new Error("Theme manifest must be a JSON object");
  }

  const obj = json as Record<string, unknown>;

  if (typeof obj.id !== "string" || obj.id.length === 0) {
    return new Error("Theme manifest must have a non-empty 'id' string");
  }
  if (typeof obj.name !== "string" || obj.name.length === 0) {
    return new Error("Theme manifest must have a non-empty 'name' string");
  }
  if (typeof obj.version !== "number") {
    return new Error("Theme manifest must have a numeric 'version'");
  }

  // Validate phaseBreakpoints if present
  if (obj.phaseBreakpoints !== undefined) {
    const bp = obj.phaseBreakpoints as Record<string, unknown>;
    if (
      typeof bp !== "object" ||
      bp === null ||
      typeof bp.mid !== "number" ||
      typeof bp.late !== "number" ||
      bp.mid >= bp.late
    ) {
      return new Error(
        "phaseBreakpoints must have numeric 'mid' and 'late' fields where mid < late",
      );
    }
  }

  const music = obj.music;
  if (typeof music !== "object" || music === null) {
    return new Error("Theme manifest must have a 'music' object");
  }

  const musicObj = music as Record<string, unknown>;
  if (!Array.isArray(musicObj.battlefield) || musicObj.battlefield.length === 0) {
    return new Error(
      "Theme manifest must have at least one track in 'music.battlefield'",
    );
  }

  // Validate track arrays
  for (const ctx of ALL_CONTEXTS) {
    const tracks = musicObj[ctx];
    if (tracks === undefined || tracks === null) continue;
    if (!Array.isArray(tracks)) {
      return new Error(`music.${ctx} must be an array of tracks`);
    }
    if (tracks.length > 100) {
      return new Error(`music.${ctx} exceeds maximum of 100 tracks`);
    }
    for (const track of tracks) {
      if (typeof track !== "object" || track === null) {
        return new Error(`Each track in music.${ctx} must be an object`);
      }
      const t = track as Record<string, unknown>;
      if (typeof t.id !== "string" || typeof t.url !== "string") {
        return new Error(
          `Each track in music.${ctx} must have 'id' and 'url' strings`,
        );
      }
    }
  }

  // Validate sfx
  if (!Array.isArray(obj.sfx)) {
    return new Error("Theme manifest must have an 'sfx' array");
  }
  for (const entry of obj.sfx as unknown[]) {
    if (typeof entry !== "object" || entry === null) {
      return new Error("Each sfx entry must be an object");
    }
    const e = entry as Record<string, unknown>;
    if (typeof e.eventType !== "string" || typeof e.url !== "string") {
      return new Error(
        "Each sfx entry must have 'eventType' and 'url' strings",
      );
    }
  }

  return json as AudioThemeManifest;
}

// ---------------------------------------------------------------------------
// Theme lookup
// ---------------------------------------------------------------------------

/**
 * Find a theme manifest by ID.
 * Checks built-in themes first, then IndexedDB cache, then custom URLs.
 * Falls back to DEFAULT_THEME if the requested theme cannot be found.
 */
export async function findManifest(
  themeId: string,
  customThemeUrls: Array<{ id: string; url: string }>,
): Promise<AudioThemeManifest> {
  const builtIn = BUILT_IN_THEMES[themeId];
  if (builtIn) return builtIn;

  // Try IndexedDB cache first
  const cached = await getCachedManifest(themeId);
  if (cached) return cached;

  // Fetch from URL
  const entry = customThemeUrls.find((e) => e.id === themeId);
  if (!entry) return DEFAULT_THEME;

  try {
    const response = await fetch(entry.url);
    if (!response.ok) return DEFAULT_THEME;
    const json: unknown = await response.json();
    const result = validateThemeManifest(json);
    if (result instanceof Error) return DEFAULT_THEME;
    return result;
  } catch {
    return DEFAULT_THEME;
  }
}
