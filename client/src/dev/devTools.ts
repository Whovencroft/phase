import type { GameState } from "../adapter/types";
import { restoreGameState } from "../game/dispatch";

declare global {
  interface Window {
    __restoreGameState?: (json: string) => void;
  }
}

window.__restoreGameState = (json: string) => {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    console.error("[DevTools] Invalid JSON");
    return;
  }

  const state = (
    parsed && typeof parsed === "object" && "gameState" in parsed
      ? (parsed as { gameState: GameState }).gameState
      : parsed
  ) as GameState;

  restoreGameState(state).then((err) => {
    if (err) {
      console.error("[DevTools] Restore failed:", err);
    } else {
      console.log("[DevTools] State restored successfully");
    }
  });
};

console.log("[DevTools] window.__restoreGameState(json) registered");
