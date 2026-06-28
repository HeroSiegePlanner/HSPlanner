import { useEffect, type RefObject } from "react";

export function useOutsideClick(
  ref: RefObject<HTMLElement | null>,
  enabled: boolean,
  onOutside: () => void,
  extraRef?: RefObject<HTMLElement | null>,
): void {
  useEffect(() => {
    if (!enabled) return;
    const handler = (e: MouseEvent | TouchEvent) => {
      const el = ref.current;
      if (!el) return;
      const target = e.target as Node;
      if (el.contains(target)) return;
      // The menu may be portaled outside `ref`; treat clicks inside it as inside.
      if (extraRef?.current?.contains(target)) return;
      onOutside();
    };
    document.addEventListener("mousedown", handler);
    document.addEventListener("touchstart", handler, { passive: true });
    return () => {
      document.removeEventListener("mousedown", handler);
      document.removeEventListener("touchstart", handler);
    };
  }, [ref, enabled, onOutside, extraRef]);
}
