import { useState, useEffect } from "react";

/** Matches the existing landscape-phone CSS media query at index.css:231 */
const COMPACT_HEIGHT_THRESHOLD = 500;

/**
 * True when the viewport is too short for default card sizing — typically
 * landscape phones (e.g., 844×390). Tablet portrait/landscape and desktop
 * stay false.
 */
export function useIsCompactHeight(): boolean {
  const [compact, setCompact] = useState(
    typeof window !== "undefined" && window.innerHeight < COMPACT_HEIGHT_THRESHOLD,
  );

  useEffect(() => {
    function handleResize() {
      setCompact(window.innerHeight < COMPACT_HEIGHT_THRESHOLD);
    }
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  return compact;
}
