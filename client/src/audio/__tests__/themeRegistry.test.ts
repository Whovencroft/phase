import { describe, expect, it } from "vitest";

import { PLANESWALKER_THEME } from "../planeswalkerTheme";
import {
  BUILT_IN_THEMES,
  DEFAULT_THEME,
  resolveTheme,
  validateThemeManifest,
} from "../themeRegistry";
import type { AudioThemeManifest } from "../types";

describe("themeRegistry", () => {
  describe("BUILT_IN_THEMES", () => {
    it("contains planeswalker as the only built-in theme", () => {
      expect(Object.keys(BUILT_IN_THEMES)).toEqual(["planeswalker"]);
    });

    it("has DEFAULT_THEME pointing to planeswalker", () => {
      expect(DEFAULT_THEME.id).toBe("planeswalker");
      expect(DEFAULT_THEME).toBe(PLANESWALKER_THEME);
    });
  });

  describe("PLANESWALKER_THEME", () => {
    it("has correct structure", () => {
      expect(PLANESWALKER_THEME.id).toBe("planeswalker");
      expect(PLANESWALKER_THEME.name).toBe("Planeswalker");
      expect(PLANESWALKER_THEME.sfx.length).toBe(15);
      expect(PLANESWALKER_THEME.music.battlefield.length).toBe(8);
    });

    it("maps all expected event types", () => {
      const eventTypes = PLANESWALKER_THEME.sfx.map((s) => s.eventType);
      expect(eventTypes).toContain("DamageDealt");
      expect(eventTypes).toContain("SpellCast");
      expect(eventTypes).toContain("CardDrawn");
      expect(eventTypes).toContain("GameStarted");
    });

    it("has tracks for all audio contexts", () => {
      expect(PLANESWALKER_THEME.music.menu?.length).toBe(2);
      expect(PLANESWALKER_THEME.music.deck_builder?.length).toBe(2);
      expect(PLANESWALKER_THEME.music.lobby?.length).toBe(1);
      expect(PLANESWALKER_THEME.music.battlefield.length).toBe(8);
      expect(PLANESWALKER_THEME.music.victory?.length).toBe(1);
      expect(PLANESWALKER_THEME.music.defeat?.length).toBe(1);
    });

    it("has phase-tagged battlefield tracks", () => {
      const phases = PLANESWALKER_THEME.music.battlefield.map((t) => t.phase);
      expect(phases).toContain("early");
      expect(phases).toContain("mid");
      expect(phases).toContain("late");
      expect(phases).toContain("any");
    });

    it("uses relative URLs for music (resolved via baseUrl)", () => {
      for (const track of PLANESWALKER_THEME.music.battlefield) {
        expect(track.url).not.toMatch(/^\//);
        expect(track.url).toMatch(/\.m4a$/);
      }
    });

    it("uses absolute URLs for SFX (bundled locally)", () => {
      for (const sfx of PLANESWALKER_THEME.sfx) {
        expect(sfx.url).toMatch(/^\/audio\/sfx\//);
      }
    });
  });

  describe("resolveTheme", () => {
    it("builds sfxMap from manifest entries", () => {
      const resolved = resolveTheme(PLANESWALKER_THEME);
      expect(resolved.sfxMap["DamageDealt"]).toBe("/audio/sfx/sfx_combat_block_001.m4a");
      expect(resolved.sfxMap["LifeGained"]).toBe("/audio/sfx/sfx_life_gain_001.m4a");
      expect(resolved.sfxMap["CardDrawn"]).toBe("/audio/sfx/sfx_card_draw_002.m4a");
    });

    it("resolves battlefield tracks with baseUrl", () => {
      const resolved = resolveTheme(PLANESWALKER_THEME);
      expect(resolved.musicByContext.battlefield.length).toBe(8);
      // Relative URLs should be resolved against baseUrl
      for (const track of resolved.musicByContext.battlefield) {
        expect(track.url).toMatch(/planeswalker-/);
        expect(track.url).toMatch(/\.m4a$/);
      }
    });

    it("resolves inheritance: deck_builder gets menu tracks", () => {
      const manifest: AudioThemeManifest = {
        id: "test",
        name: "Test",
        version: 1,
        sfx: [],
        music: {
          menu: [{ id: "menu-1", url: "/menu.mp3" }],
          battlefield: [{ id: "bf-1", url: "/bf.mp3" }],
        },
      };
      const resolved = resolveTheme(manifest);
      expect(resolved.musicByContext.deck_builder).toEqual(
        resolved.musicByContext.menu,
      );
      expect(resolved.musicByContext.lobby).toEqual(
        resolved.musicByContext.menu,
      );
    });

    it("resolves inheritance: victory/defeat get battlefield tracks", () => {
      const manifest: AudioThemeManifest = {
        id: "test",
        name: "Test",
        version: 1,
        sfx: [],
        music: {
          battlefield: [{ id: "bf-1", url: "/bf.mp3" }],
        },
      };
      const resolved = resolveTheme(manifest);
      expect(resolved.musicByContext.victory).toEqual(
        resolved.musicByContext.battlefield,
      );
      expect(resolved.musicByContext.defeat).toEqual(
        resolved.musicByContext.battlefield,
      );
    });

    it("does not inherit when context has its own tracks", () => {
      const manifest: AudioThemeManifest = {
        id: "test",
        name: "Test",
        version: 1,
        sfx: [],
        music: {
          menu: [{ id: "menu-1", url: "/menu.mp3" }],
          deck_builder: [{ id: "db-1", url: "/db.mp3" }],
          battlefield: [{ id: "bf-1", url: "/bf.mp3" }],
        },
      };
      const resolved = resolveTheme(manifest);
      expect(resolved.musicByContext.deck_builder.length).toBe(1);
      expect(resolved.musicByContext.deck_builder[0].id).toBe("db-1");
    });

    it("prepends baseUrl to relative URLs", () => {
      const manifest: AudioThemeManifest = {
        id: "test",
        name: "Test",
        version: 1,
        baseUrl: "https://cdn.example.com/audio",
        sfx: [{ eventType: "DamageDealt", url: "sfx/damage.mp3" }],
        music: {
          battlefield: [{ id: "bf-1", url: "music/battle.mp3" }],
        },
      };
      const resolved = resolveTheme(manifest);
      expect(resolved.sfxMap["DamageDealt"]).toBe(
        "https://cdn.example.com/audio/sfx/damage.mp3",
      );
      expect(resolved.musicByContext.battlefield[0].url).toBe(
        "https://cdn.example.com/audio/music/battle.mp3",
      );
    });

    it("does not prepend baseUrl to absolute URLs", () => {
      const manifest: AudioThemeManifest = {
        id: "test",
        name: "Test",
        version: 1,
        baseUrl: "https://cdn.example.com",
        sfx: [{ eventType: "DamageDealt", url: "https://other.com/sfx.mp3" }],
        music: {
          battlefield: [{ id: "bf-1", url: "/local/battle.mp3" }],
        },
      };
      const resolved = resolveTheme(manifest);
      expect(resolved.sfxMap["DamageDealt"]).toBe("https://other.com/sfx.mp3");
      expect(resolved.musicByContext.battlefield[0].url).toBe(
        "/local/battle.mp3",
      );
    });

    it("defaults track phase to 'any'", () => {
      const manifest: AudioThemeManifest = {
        id: "test",
        name: "Test",
        version: 1,
        sfx: [],
        music: {
          battlefield: [{ id: "bf-1", url: "/bf.mp3" }],
        },
      };
      const resolved = resolveTheme(manifest);
      expect(resolved.musicByContext.battlefield[0].phase).toBe("any");
    });
  });

  describe("validateThemeManifest", () => {
    const validManifest = {
      id: "test",
      name: "Test Theme",
      version: 1,
      sfx: [{ eventType: "DamageDealt", url: "/sfx/damage.mp3" }],
      music: {
        battlefield: [{ id: "bf-1", url: "/music/battle.mp3" }],
      },
    };

    it("accepts a valid manifest", () => {
      const result = validateThemeManifest(validManifest);
      expect(result).not.toBeInstanceOf(Error);
      expect((result as AudioThemeManifest).id).toBe("test");
    });

    it("rejects non-object input", () => {
      expect(validateThemeManifest("not an object")).toBeInstanceOf(Error);
      expect(validateThemeManifest(null)).toBeInstanceOf(Error);
    });

    it("rejects missing id", () => {
      expect(
        validateThemeManifest({ ...validManifest, id: "" }),
      ).toBeInstanceOf(Error);
    });

    it("rejects missing battlefield tracks", () => {
      expect(
        validateThemeManifest({
          ...validManifest,
          music: { battlefield: [] },
        }),
      ).toBeInstanceOf(Error);
    });

    it("rejects invalid sfx entries", () => {
      expect(
        validateThemeManifest({
          ...validManifest,
          sfx: [{ eventType: 123, url: "/sfx.mp3" }],
        }),
      ).toBeInstanceOf(Error);
    });

    it("rejects track array exceeding 100 entries", () => {
      const bigBattlefield = Array.from({ length: 101 }, (_, i) => ({
        id: `track-${i}`,
        url: `/music/${i}.mp3`,
      }));
      expect(
        validateThemeManifest({
          ...validManifest,
          music: { battlefield: bigBattlefield },
        }),
      ).toBeInstanceOf(Error);
    });
  });
});
