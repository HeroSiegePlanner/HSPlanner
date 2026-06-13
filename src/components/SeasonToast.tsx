import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { EASE_OUT } from "../lib/motion";
import { useBuild } from "../store/build";
import { activeSeasonId } from "../data";
import { getSeason } from "../data/seasons/registry";

const DISMISS_MS = 3500;

// Transient toast confirming the open build's season (activeSeasonId, via reload-on-open).
export default function SeasonToast() {
  const activeBuildId = useBuild((s) => s.activeBuildId);
  const [visible, setVisible] = useState(false);
  const seasonName = getSeason(activeSeasonId)?.name ?? activeSeasonId;

  useEffect(() => {
    if (!activeBuildId) return;
    // Effect-driven toast on build change; the auto-hide timer bounds the re-render.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setVisible(true);
    const timer = window.setTimeout(() => setVisible(false), DISMISS_MS);
    return () => window.clearTimeout(timer);
  }, [activeBuildId]);

  return (
    <AnimatePresence>
      {visible && (
        <motion.div
          role="status"
          aria-live="polite"
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: 10 }}
          transition={{ duration: 0.18, ease: EASE_OUT }}
          className="pointer-events-none fixed bottom-4 right-4 z-50 flex items-center gap-2.5 rounded-[3px] border border-border-2 px-3 py-2"
          style={{
            background:
              "linear-gradient(180deg, var(--color-panel-2), var(--color-panel))",
            boxShadow:
              "inset 0 1px 0 rgba(201,165,90,0.08), 0 6px 20px rgba(0,0,0,0.5)",
          }}
        >
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: "0 0 8px rgba(224,184,100,0.6)" }}
          />
          <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
            This build is on
          </span>
          <span
            className="font-mono text-[12px] uppercase tracking-[0.16em] text-accent-hot"
            style={{ textShadow: "0 0 10px rgba(224,184,100,0.25)" }}
          >
            {seasonName}
          </span>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
