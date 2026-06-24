import { useEffect } from "react";

import { initializeFeeds } from "../services/feedService";

export function useFeedInitialization(): void {
  useEffect(() => {
    initializeFeeds().catch((err) => {
      console.error("Feed initialization failed:", err);
    });
  }, []);
}
