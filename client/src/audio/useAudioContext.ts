import { useEffect } from "react";

import { audioManager } from "./AudioManager";
import type { AudioContextName } from "./types";

/**
 * Set the audio context for the current page.
 *
 * No cleanup on unmount — context transitions are driven by the next page's
 * setContext() call. This avoids race conditions where React effect cleanup
 * fires after the new page's mount effect, reverting the context.
 */
export function useAudioContext(context: AudioContextName): void {
  useEffect(() => {
    audioManager.setContext(context);
  }, [context]);
}
