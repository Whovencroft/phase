import { useEffect, useRef } from "react";

import { useUiStore } from "../stores/uiStore.ts";

/**
 * Safety mechanism to dismiss the card preview when the pointer is no longer
 * over an inspectable element. This handles the case where framer-motion
 * animations (tap rotation, attack slide, layout transitions) move elements
 * out from under the cursor without firing onMouseLeave.
 *
 * Uses `document.elementFromPoint()` on a 300ms interval to verify the pointer
 * is still over an element with `[data-card-hover]`.
 *
 * When `previewSticky` is true (set by long-press on touch devices), the
 * interval-based dismiss is skipped. Instead, a global touchstart listener
 * dismisses the preview when the user taps outside a card-hover element.
 */
export function usePreviewDismiss() {
  const inspectedObjectId = useUiStore((s) => s.inspectedObjectId);
  const previewSticky = useUiStore((s) => s.previewSticky);
  const inspectObject = useUiStore((s) => s.inspectObject);
  const hoverObject = useUiStore((s) => s.hoverObject);
  const pointerRef = useRef({ x: 0, y: 0 });

  // Track pointer position (only while preview is active, mouse only)
  useEffect(() => {
    if (inspectedObjectId == null) return;

    function onMove(e: PointerEvent) {
      // Only track mouse pointer, not touch — touch uses sticky dismiss
      if (e.pointerType === "touch") return;
      pointerRef.current = { x: e.clientX, y: e.clientY };
    }

    document.addEventListener("pointermove", onMove, { passive: true });
    return () => document.removeEventListener("pointermove", onMove);
  }, [inspectedObjectId]);

  // Mouse: periodically verify the pointer is still over a card-hover element
  // Skipped when preview is sticky (touch-initiated)
  useEffect(() => {
    if (inspectedObjectId == null || previewSticky) return;

    let skipFirst = true;

    const id = setInterval(() => {
      if (skipFirst) {
        skipFirst = false;
        return;
      }
      const { x, y } = pointerRef.current;
      if (x === 0 && y === 0) return;

      const el = document.elementFromPoint(x, y);
      if (!el) return;

      const isOverCard = el.closest("[data-card-hover]") !== null;
      if (!isOverCard) {
        inspectObject(null);
        hoverObject(null);
      }
    }, 300);

    return () => clearInterval(id);
  }, [inspectedObjectId, previewSticky, inspectObject, hoverObject]);

  // Touch: tap outside a card-hover element dismisses sticky preview
  useEffect(() => {
    if (inspectedObjectId == null || !previewSticky) return;

    function onTouch(e: TouchEvent) {
      const touch = e.touches[0];
      if (!touch) return;
      const el = document.elementFromPoint(touch.clientX, touch.clientY);
      if (!el) return;

      // Don't dismiss if tapping on a card or the preview itself
      const isOverCard = el.closest("[data-card-hover]") !== null;
      const isOverPreview = el.closest("[data-card-preview]") !== null;
      if (!isOverCard && !isOverPreview) {
        inspectObject(null);
        hoverObject(null);
      }
    }

    // Use a small delay so the touchstart that opened the preview doesn't immediately dismiss
    const timer = setTimeout(() => {
      document.addEventListener("touchstart", onTouch, { passive: true });
    }, 100);

    return () => {
      clearTimeout(timer);
      document.removeEventListener("touchstart", onTouch);
    };
  }, [inspectedObjectId, previewSticky, inspectObject, hoverObject]);
}
