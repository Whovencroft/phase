/** Audio contexts corresponding to distinct pages/states where music plays. */
export type AudioContextName =
  | "menu"
  | "deck_builder"
  | "lobby"
  | "battlefield"
  | "victory"
  | "defeat";

/** Phase tag for battlefield tracks — enables conditional track selection by game progression. */
export type GamePhaseTag = "early" | "mid" | "late" | "any";

/** A single music track within a theme. */
export interface ThemeTrack {
  id: string;
  url: string;
  /** Battlefield phase this track is associated with. Defaults to "any". */
  phase?: GamePhaseTag;
  label?: string;
}

/** A single SFX entry mapping a GameEvent type to an audio file. */
export interface ThemeSfx {
  /** GameEvent type string (e.g., "DamageDealt", "SpellCast"). */
  eventType: string;
  url: string;
}

/** Complete theme manifest — a JSON-serializable description of an audio theme. */
export interface AudioThemeManifest {
  id: string;
  name: string;
  version: number;
  author?: string;
  description?: string;
  /** Base URL prefix prepended to relative asset URLs. */
  baseUrl?: string;
  /** Turn thresholds for battlefield phase transitions. Defaults: { mid: 5, late: 10 }. */
  phaseBreakpoints?: { mid: number; late: number };
  sfx: ThemeSfx[];
  music: {
    menu?: ThemeTrack[];
    deck_builder?: ThemeTrack[];
    lobby?: ThemeTrack[];
    battlefield: ThemeTrack[];
    victory?: ThemeTrack[];
    defeat?: ThemeTrack[];
  };
}

/** Runtime-resolved theme with inheritance applied and lookup maps built. */
export interface ResolvedTheme {
  manifest: AudioThemeManifest;
  /** eventType → resolved URL for SFX lookup. */
  sfxMap: Record<string, string>;
  /** Final resolved track list per context (inheritance already applied). */
  musicByContext: Record<AudioContextName, ThemeTrack[]>;
}
