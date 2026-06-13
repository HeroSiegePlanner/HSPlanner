import { useState } from "react";
import { motion } from "motion/react";
import { hoverTap } from "../lib/motion";
import { getSeason, SEASONS, setStoredSeasonId } from "../data/seasons/registry";
import { activeSeasonId } from "../data";
import {
  MODAL_BTN_CLASS,
  MODAL_BTN_PRIMARY_CLASS,
  MODAL_FOOTER_CLASS,
  Modal,
} from "./Modal";

export default function SeasonSwitcher() {
  const [pending, setPending] = useState<string | null>(null);
  const pendingSeason = pending ? getSeason(pending) : undefined;

  const confirm = () => {
    if (!pending) return;
    if (setStoredSeasonId(pending)) {
      window.location.reload();
      return;
    }
    setPending(null);
  };

  return (
    <>
      <label className="flex items-center gap-2">
        <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          Season
        </span>
        <div
          className="inline-flex items-center rounded-[3px] border border-border-2 px-2 py-1 transition-colors hover:border-accent-deep focus-within:border-accent-hot"
          style={{
            background: "linear-gradient(180deg, #0d0e12, var(--color-panel-2))",
            boxShadow: "inset 0 1px 2px rgba(0,0,0,0.5)",
          }}
        >
          <select
            value={activeSeasonId}
            onChange={(e) => {
              if (e.target.value !== activeSeasonId) setPending(e.target.value);
            }}
            className="min-w-20 cursor-pointer bg-transparent text-[12px] text-text outline-none"
          >
            {SEASONS.map((s) => (
              <option key={s.id} value={s.id} className="bg-panel">
                {s.name}
              </option>
            ))}
          </select>
        </div>
      </label>
      {pendingSeason && (
        <Modal
          onClose={() => setPending(null)}
          panelClassName="w-[440px] max-w-[92vw]"
          eyebrow="Season"
          title="Switch season?"
          titleId="season-switch-title"
        >
          <section className="px-6 py-4">
            <p className="m-0 font-mono text-[12px] leading-relaxed tracking-[0.04em] text-muted">
              The app will reload with{" "}
              <span className="text-accent-hot">{pendingSeason.name}</span>{" "}
              data. Builds from another season are converted in place when
              opened — removed parts will be reported.
            </p>
          </section>
          <footer
            className={MODAL_FOOTER_CLASS}
            style={{ background: "rgba(0,0,0,0.3)" }}
          >
            <motion.button
              {...hoverTap}
              type="button"
              onClick={() => setPending(null)}
              className={MODAL_BTN_CLASS}
            >
              Cancel
            </motion.button>
            <motion.button
              {...hoverTap}
              type="button"
              onClick={confirm}
              className={MODAL_BTN_PRIMARY_CLASS}
              style={{
                background:
                  "linear-gradient(180deg, rgba(58,46,24,0.6), rgba(42,36,24,0.4))",
              }}
            >
              Switch &amp; reload
            </motion.button>
          </footer>
        </Modal>
      )}
    </>
  );
}
