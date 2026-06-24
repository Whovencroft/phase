import { useEffect, useState } from "react";

interface ScryfallSetEntry {
  icon_svg_uri?: string;
}

type SetSymbols = Record<string, ScryfallSetEntry>;

let cached: SetSymbols | null = null;
let fetchPromise: Promise<SetSymbols | null> | null = null;

function fetchSetSymbols(): Promise<SetSymbols | null> {
  if (!fetchPromise) {
    fetchPromise = fetch(__SCRYFALL_SETS_URL__)
      .then((res) => (res.ok ? (res.json() as Promise<SetSymbols>) : null))
      .then((data) => {
        if (data && typeof data === "object") cached = data;
        return cached;
      })
      .catch(() => null);
  }
  return fetchPromise;
}

export function useSetSymbol(setCode: string | undefined): string | null {
  const [symbols, setSymbols] = useState<SetSymbols | null>(cached);

  useEffect(() => {
    if (cached) return;
    fetchSetSymbols().then((data) => { if (data) setSymbols(data); });
  }, []);

  if (!setCode) return null;
  return symbols?.[setCode.toLowerCase()]?.icon_svg_uri ?? null;
}
