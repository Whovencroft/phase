import { useEffect, useMemo, useState } from "react";

import { getCachedFeed, listSubscriptions, feedDeckToParsedDeck } from "../services/feedService";
import type { FeedDeck } from "../types/feed";
import { evaluateDeckCompatibilityBatch } from "../services/deckCompatibility";
import { classifyDeck, type DeckArchetype } from "../services/engineRuntime";

export interface FeedDeckMeta {
  archetype: DeckArchetype | null;
  coveragePct: number | null;
}

export interface FeedDeckList {
  decks: FeedDeck[];
  meta: Map<string, FeedDeckMeta>;
  loading: boolean;
}

function collectFeedDecks(format?: string): FeedDeck[] {
  const normalized = format?.toLowerCase();
  const out: FeedDeck[] = [];
  for (const sub of listSubscriptions()) {
    const feed = getCachedFeed(sub.sourceId);
    if (!feed) continue;
    if (normalized && feed.format !== normalized) continue;
    out.push(...feed.decks);
  }
  return out;
}

function coveragePct(total: number, supported: number): number | null {
  if (total <= 0) return null;
  return Math.round((supported / total) * 100);
}

/** Surfaces the format-scoped feed deck list with pre-computed coverage + archetype
 *  metadata. Computes once per deck name and caches across re-renders; slider drag
 *  on the UI side is a pure filter over this map. */
export function useFeedDeckList(format?: string): FeedDeckList {
  const decks = useMemo(() => collectFeedDecks(format), [format]);
  const [meta, setMeta] = useState<Map<string, FeedDeckMeta>>(new Map());
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (decks.length === 0) {
      setMeta(new Map());
      setLoading(false);
      return;
    }

    let cancelled = false;
    setLoading(true);

    (async () => {
      const parsedEntries = decks.map((d) => ({ name: d.name, deck: feedDeckToParsedDeck(d) }));
      const compat = await evaluateDeckCompatibilityBatch(parsedEntries);

      const archetypes = await Promise.all(
        decks.map(async (d) => {
          const names: string[] = [];
          for (const entry of d.main) {
            for (let i = 0; i < entry.count; i++) names.push(entry.name);
          }
          try {
            const profile = await classifyDeck(names);
            return profile.archetype;
          } catch {
            return null;
          }
        }),
      );

      if (cancelled) return;

      const next = new Map<string, FeedDeckMeta>();
      decks.forEach((d, i) => {
        const cov = compat[d.name]?.coverage;
        next.set(d.name, {
          archetype: archetypes[i] ?? null,
          coveragePct: cov ? coveragePct(cov.total_unique, cov.supported_unique) : null,
        });
      });
      setMeta(next);
      setLoading(false);
    })();

    return () => {
      cancelled = true;
    };
  }, [decks]);

  return { decks, meta, loading };
}
