import { motion } from "motion/react";
import { hoverTap } from "../lib/motion";
import { getSeason } from "../data/seasons/registry";
import { useBuild } from "../store/build";
import { Modal } from "./Modal";

function Row({ label, entries }: { label: string; entries: string[] }) {
  if (entries.length === 0) return null;
  return (
    <div className="flex flex-col gap-1">
      <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
        {label}
      </span>
      <span className="text-[12px] text-muted">{entries.join(", ")}</span>
    </div>
  );
}

export default function SeasonConversionModal() {
  const report = useBuild((s) => s.seasonConversionReport);
  const clear = useBuild((s) => s.clearSeasonConversionReport);
  if (!report || !report.hasChanges) return null;

  const from = getSeason(report.fromSeason)?.name ?? report.fromSeason;
  const to = getSeason(report.toSeason)?.name ?? report.toSeason;

  return (
    <Modal
      onClose={clear}
      panelClassName="max-h-[80vh] w-[560px] max-w-[94vw]"
      eyebrow="Season"
      title={`Build converted: ${from} → ${to}`}
      titleId="season-conversion-title"
    >
      <section className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-6 py-4">
        <Row
          label="Removed tree nodes"
          entries={report.removedTreeNodes.map(String)}
        />
        <Row
          label="Orphaned tree nodes (pruned)"
          entries={report.orphanedTreeNodes.map(String)}
        />
        {report.freedTreePoints > 0 && (
          <Row
            label="Freed tree points (affects level)"
            entries={[String(report.freedTreePoints)]}
          />
        )}
        <Row
          label="Removed items"
          entries={report.removedItems.map((e) => `${e.slot}: ${e.baseId}`)}
        />
        <Row
          label="Removed affixes"
          entries={report.removedAffixes.map((e) => `${e.slot}: ${e.affixId}`)}
        />
        <Row
          label="Removed forged mods"
          entries={report.removedForgedMods.map(
            (e) => `${e.slot}: ${e.affixId}`,
          )}
        />
        <Row
          label="Removed socketables"
          entries={report.removedSocketables.map((e) => `${e.slot}: ${e.id}`)}
        />
        <Row
          label="Removed runewords"
          entries={report.removedRunewords.map(
            (e) => `${e.slot}: ${e.runewordId}`,
          )}
        />
        <Row
          label="Removed augments"
          entries={report.removedAugments.map((e) => `${e.slot}: ${e.id}`)}
        />
        <Row label="Removed skills" entries={report.removedSkills} />
        <Row label="Removed subskills" entries={report.removedSubskills} />
        <Row
          label="Cleared tree sockets"
          entries={report.removedTreeSockets.map(String)}
        />
        <Row
          label="Removed jewel affixes"
          entries={report.removedUncutAffixes.map(
            (e) => `${e.nodeId}: ${e.affixId}`,
          )}
        />
      </section>
      <footer
        className="flex items-center justify-end gap-2 border-t border-border px-6 py-3"
        style={{ background: "rgba(0,0,0,0.3)" }}
      >
        <motion.button
          {...hoverTap}
          type="button"
          onClick={clear}
          className="rounded-[3px] border border-accent-deep px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot"
          style={{
            background:
              "linear-gradient(180deg, rgba(58,46,24,0.6), rgba(42,36,24,0.4))",
          }}
        >
          OK
        </motion.button>
      </footer>
    </Modal>
  );
}
