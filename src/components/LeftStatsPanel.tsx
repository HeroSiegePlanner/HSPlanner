import { useEffect, useMemo, useState } from "react";
import { gameConfig, getClass, getSkillsByClass } from "../data";
import { isImageUrl } from "../utils/imageUrl";
import {
  attrPointsFor,
  skillPointsFor,
  useBuild,
} from "../store/build";
import {
  aggregateItemSkillBonuses,
  combineAdditiveAndMore,
  effectiveCap,
  formatValue,
  isZero,
  manaCostAtRank,
  normalizeSkillName,
  rangedMax,
  rangedMin,
  statDef,
} from "../utils/item/stats";
import { computeBuildPerformanceAsync } from "../lib/calc/bridge";
import type { BuildPerformance } from "../utils/build/buildPerformance";
import { useBuildPerformanceDeps } from "../hooks/useBuildPerformanceDeps";
import type { RangedValue } from "../types";

const ATTRIBUTE_ORDER: string[] = [
  "strength",
  "dexterity",
  "intelligence",
  "energy",
  "vitality",
  "armor",
];

const OFFENSE_KEYS = [
  "enhanced_damage",
  "attack_damage",
  "increased_attack_speed",
  "faster_cast_rate",
  "crit_chance",
  "crit_damage",
  "life_steal",
  "mana_steal",
];

const DEFENSE_KEYS = [
  "life",
  "mana",
  "life_replenish",
  "mana_replenish",
  "block_chance",
  "physical_damage_reduction",
  "magic_damage_reduction",
];

interface ResistanceStyle {
  key: string;
  label: string;
  className: string;
}

const RESISTANCES: ResistanceStyle[] = [
  { key: "fire_resistance", label: "Fire", className: "text-stat-red" },
  { key: "cold_resistance", label: "Cold", className: "text-stat-blue" },
  {
    key: "lightning_resistance",
    label: "Lightning",
    className: "text-stat-orange",
  },
  { key: "poison_resistance", label: "Poison", className: "text-stat-purple" },
];

const ATTR_COLOR: Record<string, string> = {
  strength: "text-stat-orange",
  dexterity: "text-stat-green",
  intelligence: "text-stat-purple",
  energy: "text-stat-blue",
  vitality: "text-stat-red",
  armor: "text-muted",
};

const GOLD_OFFENSE = new Set(["enhanced_damage", "crit_chance", "crit_damage"]);
const GOLD_DEFENSE = new Set(["life"]);
const BLUE_DEFENSE = new Set(["mana", "mana_replenish"]);

// Fold `<key>_more` Total multiplier into the additive sum so display matches engine output.
function effectiveStatValue(
  stats: Record<string, RangedValue>,
  key: string,
): RangedValue {
  const additive = stats[key];
  const more = stats[`${key}_more`];
  if (more === undefined) return additive ?? 0;
  return combineAdditiveAndMore(additive, more);
}

export default function LeftStatsPanel() {
  const classId = useBuild((s) => s.classId);
  const level = useBuild((s) => s.level);
  const allocated = useBuild((s) => s.allocated);
  const inventory = useBuild((s) => s.inventory);
  const skillRanks = useBuild((s) => s.skillRanks);
  const mainSkillId = useBuild((s) => s.mainSkillId);
  const setMainSkill = useBuild((s) => s.setMainSkill);
  const activeAuraId = useBuild((s) => s.activeAuraId);
  const setActiveAura = useBuild((s) => s.setActiveAura);

  const buildDeps = useBuildPerformanceDeps();
  const [performance, setPerformance] = useState<BuildPerformance | null>(null);
  useEffect(() => {
    let cancelled = false;
    computeBuildPerformanceAsync(buildDeps).then((p) => {
      if (!cancelled) setPerformance(p);
    });
    return () => {
      cancelled = true;
    };
  }, [buildDeps]);
  const attributes = performance?.attributes ?? {};
  const stats = performance?.stats ?? {};
  const damage = performance?.damage ?? null;
  const hitDpsMin = performance?.hitDpsMin;
  const hitDpsMax = performance?.hitDpsMax;
  const combinedDpsMin = performance?.combinedDpsMin;
  const combinedDpsMax = performance?.combinedDpsMax;

  const cls = classId ? getClass(classId) : undefined;
  const attrSpent = Object.values(allocated).reduce((s, v) => s + v, 0);
  const attrTotal = attrPointsFor(level);
  const skillSpent = Object.values(skillRanks).reduce((s, v) => s + v, 0);
  const skillTotal = skillPointsFor(level);
  const heroLevel = buildDeps.allocatedTreeNodes.size;

  const allClassSkills = useMemo(() => getSkillsByClass(classId), [classId]);
  const classSkills = useMemo(
    () => allClassSkills.filter((s) => s.kind === "active"),
    [allClassSkills],
  );
  const classAuras = useMemo(
    () => allClassSkills.filter((s) => s.kind === "aura"),
    [allClassSkills],
  );
  const activeSkill =
    mainSkillId != null ? classSkills.find((s) => s.id === mainSkillId) : null;
  const activeRank = activeSkill ? (skillRanks[activeSkill.id] ?? 0) : 0;

  const itemSkillBonuses = useMemo(
    () => aggregateItemSkillBonuses(inventory),
    [inventory],
  );
  const allSkillsMin = rangedMin(stats.all_skills ?? 0);
  const allSkillsMax = rangedMax(stats.all_skills ?? 0);
  const elementSkillsMin = activeSkill?.damageType
    ? rangedMin(stats[`${activeSkill.damageType}_skills`] ?? 0)
    : 0;
  const elementSkillsMax = activeSkill?.damageType
    ? rangedMax(stats[`${activeSkill.damageType}_skills`] ?? 0)
    : 0;
  const itemBonus: [number, number] = activeSkill
    ? (itemSkillBonuses[normalizeSkillName(activeSkill.name)] ?? [0, 0])
    : [0, 0];
  const rankBonusMin = allSkillsMin + elementSkillsMin + itemBonus[0];
  const rankBonusMax = allSkillsMax + elementSkillsMax + itemBonus[1];
  const effRankMin = activeRank + rankBonusMin;
  const effRankMax = activeRank + rankBonusMax;

  const baseManaMin = activeSkill
    ? manaCostAtRank(activeSkill, Math.max(effRankMin, 1))
    : undefined;
  const baseManaMax = activeSkill
    ? manaCostAtRank(activeSkill, Math.max(effRankMax, 1))
    : undefined;
  const fcrCombined = combineAdditiveAndMore(
    stats.faster_cast_rate,
    stats.faster_cast_rate_more,
  );
  const fcrMin = rangedMin(fcrCombined);
  const fcrMax = rangedMax(fcrCombined);
  const mcrMin = rangedMin(stats.mana_cost_reduction ?? 0);
  const mcrMax = rangedMax(stats.mana_cost_reduction ?? 0);
  const effCastMin = activeSkill?.baseCastRate
    ? activeSkill.baseCastRate * (1 + fcrMin / 100)
    : undefined;
  const effCastMax = activeSkill?.baseCastRate
    ? activeSkill.baseCastRate * (1 + fcrMax / 100)
    : undefined;
  const effManaMin =
    baseManaMin !== undefined ? baseManaMin * (1 - mcrMax / 100) : undefined;
  const effManaMax =
    baseManaMax !== undefined ? baseManaMax * (1 - mcrMin / 100) : undefined;
  const manaPerSecMin =
    effManaMin !== undefined && effCastMin !== undefined
      ? effManaMin * effCastMin
      : undefined;
  const manaPerSecMax =
    effManaMax !== undefined && effCastMax !== undefined
      ? effManaMax * effCastMax
      : undefined;
  const manaRegenCombined = combineAdditiveAndMore(
    stats.mana_replenish,
    stats.mana_replenish_more,
  );
  const manaRegenMin = rangedMin(manaRegenCombined);
  const manaRegenMax = rangedMax(manaRegenCombined);
  const sustainable =
    manaPerSecMax !== undefined && manaPerSecMax <= manaRegenMin;
  const unsustainable =
    manaPerSecMin !== undefined && manaPerSecMin > manaRegenMax;
  const netMin =
    manaPerSecMax !== undefined ? manaRegenMin - manaPerSecMax : undefined;
  const netMax =
    manaPerSecMin !== undefined ? manaRegenMax - manaPerSecMin : undefined;
  const uptimeMin =
    manaPerSecMax !== undefined
      ? manaPerSecMax <= 0
        ? 100
        : Math.min(100, (manaRegenMin / manaPerSecMax) * 100)
      : undefined;
  const uptimeMax =
    manaPerSecMin !== undefined
      ? manaPerSecMin <= 0
        ? 100
        : Math.min(100, (manaRegenMax / manaPerSecMin) * 100)
      : undefined;

  return (
    <aside
      className="relative flex h-full w-72 shrink-0 flex-col overflow-y-auto border-r border-border text-[12px]"
      style={{
        background:
          "linear-gradient(180deg, var(--color-panel-2), var(--color-panel) 40%, var(--color-bg))",
        boxShadow: "inset -1px 0 0 rgba(201,165,90,0.05)",
      }}
    >
      <div
        className="border-b border-border px-4 py-3"
        style={{
          background:
            "linear-gradient(180deg, rgba(201,165,90,0.05), transparent)",
        }}
      >
        <div className="mb-1 flex items-start gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          <span
            aria-hidden
            className="mt-1 inline-block h-1.5 w-1.5 shrink-0 rotate-45 bg-accent-hot"
            style={{ boxShadow: "0 0 8px rgba(224,184,100,0.6)" }}
          />
          <span className="mt-0.5">Character</span>
          <span className="ml-auto flex flex-col items-end leading-tight text-accent-hot">
            <span>Lv {level}</span>
            <span>Hero Lv {heroLevel}</span>
          </span>
        </div>
        <div
          className="text-[15px] font-semibold tracking-[0.02em] text-accent-hot"
          style={{ textShadow: "0 0 14px rgba(224,184,100,0.18)" }}
        >
          {cls?.name ?? "No class"}
        </div>
        {cls?.primaryAttribute && (
          <div className="mt-0.5 font-mono text-[9px] uppercase tracking-[0.18em] text-accent-deep">
            Primary · {cls.primaryAttribute}
          </div>
        )}
      </div>

      {classAuras.length > 0 && (
        <Section title="Active Aura">
          <PanelSelect
            value={activeAuraId ?? ""}
            onChange={(e) => setActiveAura(e.target.value || null)}
          >
            <option value="">— none —</option>
            {classAuras.map((s) => {
              const rank = skillRanks[s.id] ?? 0;
              return (
                <option key={s.id} value={s.id} disabled={rank === 0}>
                  {s.icon && !isImageUrl(s.icon) ? `${s.icon} ` : ""}
                  {s.name}
                </option>
              );
            })}
          </PanelSelect>
        </Section>
      )}

      <Section title="Main Skill">
        {classSkills.length === 0 ? (
          <div className="font-mono text-[11px] tracking-[0.04em] text-muted italic">
            No skills for this class
          </div>
        ) : (
          <>
            <PanelSelect
              value={mainSkillId ?? ""}
              onChange={(e) => setMainSkill(e.target.value || null)}
              className="mb-2"
            >
              <option value="">— select —</option>
              {classSkills.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.icon && !isImageUrl(s.icon) ? `${s.icon} ` : ""}
                  {s.name}
                </option>
              ))}
            </PanelSelect>
            {activeSkill && (
              <>
                <Row
                  label="Rank"
                  value={
                    <>
                      <span className="text-text">
                        {effRankMin === effRankMax
                          ? effRankMin
                          : `${effRankMin}-${effRankMax}`}
                      </span>
                      {(rankBonusMin !== 0 || rankBonusMax !== 0) && (
                        <span className="text-accent">
                          {" "}
                          ({activeRank}
                          {rankBonusMin === rankBonusMax
                            ? rankBonusMin >= 0
                              ? `+${rankBonusMin}`
                              : rankBonusMin
                            : ` +${rankBonusMin}-${rankBonusMax}`}
                          )
                        </span>
                      )}
                      <span className="text-muted">/{activeSkill.maxRank}</span>
                    </>
                  }
                />
                <Row
                  label="Mana / cast"
                  value={
                    effManaMin === undefined || effManaMax === undefined ? (
                      <span className="text-muted">—</span>
                    ) : (
                      <span className="text-stat-blue">
                        {formatNumRange(effManaMin, effManaMax)}
                      </span>
                    )
                  }
                />
                <Row
                  label="Cast rate"
                  value={
                    effCastMin !== undefined && effCastMax !== undefined ? (
                      <span className="text-text">
                        {formatNumRange(effCastMin, effCastMax)}/s
                      </span>
                    ) : (
                      <span className="text-muted">—</span>
                    )
                  }
                />
                <Row
                  label="Mana / sec"
                  value={
                    manaPerSecMin !== undefined &&
                    manaPerSecMax !== undefined ? (
                      <span
                        className={
                          sustainable
                            ? "text-stat-green"
                            : unsustainable
                              ? "text-stat-red"
                              : "text-stat-orange"
                        }
                      >
                        {formatNumRange(manaPerSecMin, manaPerSecMax)}
                      </span>
                    ) : (
                      <span className="text-muted">—</span>
                    )
                  }
                />
                <Row
                  label="Mana regen"
                  value={
                    <span className="text-stat-blue">
                      {formatNumRange(manaRegenMin, manaRegenMax)}
                    </span>
                  }
                />
                {netMin !== undefined && netMax !== undefined && (
                  <Row
                    label="Net mana / sec"
                    value={
                      <span
                        className={
                          netMin >= 0
                            ? "text-stat-green"
                            : netMax < 0
                              ? "text-stat-red"
                              : "text-stat-orange"
                        }
                      >
                        {netMin >= 0 ? "+" : ""}
                        {formatNumRange(netMin, netMax)}
                      </span>
                    }
                  />
                )}
                {uptimeMin !== undefined && uptimeMax !== undefined && (
                  <Row
                    label="Uptime"
                    value={
                      <span
                        className={
                          uptimeMin >= 100
                            ? "text-stat-green"
                            : uptimeMax < 75
                              ? "text-stat-red"
                              : "text-stat-orange"
                        }
                      >
                        {formatNumRange(
                          Math.round(uptimeMin),
                          Math.round(uptimeMax),
                        )}
                        %
                      </span>
                    }
                  />
                )}
                <div className="my-2 border-t border-dashed border-accent-deep/30" />
                <Row
                  label="Hit damage"
                  value={
                    damage ? (
                      <span className="text-text">
                        {formatNumRange(damage.finalMin, damage.finalMax)}
                      </span>
                    ) : (
                      <span className="text-muted">—</span>
                    )
                  }
                />
                <Row
                  label="Hit DPS"
                  value={
                    hitDpsMin !== undefined && hitDpsMax !== undefined ? (
                      <span className="text-accent-hot">
                        {formatNumRange(
                          Math.round(hitDpsMin),
                          Math.round(hitDpsMax),
                        )}
                      </span>
                    ) : (
                      <span className="text-muted">—</span>
                    )
                  }
                />
                <Row
                  label="Combined DPS"
                  value={
                    combinedDpsMin !== undefined &&
                    combinedDpsMax !== undefined ? (
                      <span
                        className="font-semibold text-accent-hot"
                        style={{
                          textShadow:
                            "0 0 10px rgba(224,184,100,0.25)",
                        }}
                      >
                        {formatNumRange(
                          Math.round(combinedDpsMin),
                          Math.round(combinedDpsMax),
                        )}
                      </span>
                    ) : (
                      <span className="text-muted">—</span>
                    )
                  }
                />
              </>
            )}
          </>
        )}
      </Section>

      <Section title="Points">
        <Row
          label="Attr used"
          value={
            <>
              <span className="text-text">{attrSpent}</span>
              <span className="text-muted">/{attrTotal}</span>
            </>
          }
        />
        <Row
          label="Skill used"
          value={
            <>
              <span className="text-text">{skillSpent}</span>
              <span className="text-muted">/{skillTotal}</span>
            </>
          }
        />
        <Row
          label="Tree nodes"
          value={<span className="text-text">{buildDeps.allocatedTreeNodes.size}</span>}
        />
      </Section>

      <Section title="Attributes">
        {ATTRIBUTE_ORDER.map((key) => {
          const attr = gameConfig.attributes.find((a) => a.key === key);
          if (!attr) return null;
          const v = attributes[attr.key];
          const color = ATTR_COLOR[key] ?? "text-text";
          return (
            <div
              key={key}
              className="flex items-baseline justify-between gap-2 py-0.75"
            >
              <span className={`${color} flex-1 min-w-0 leading-tight`}>
                {attr.name}
              </span>
              <span
                className={`font-mono tabular-nums shrink-0 whitespace-nowrap text-right ${color}`}
              >
                {formatValue(v ?? 0, key)}
              </span>
            </div>
          );
        })}
      </Section>

      <Section title="Offense">
        {OFFENSE_KEYS.map((key) => (
          <StatLine
            key={key}
            statKey={key}
            value={effectiveStatValue(stats, key)}
            highlight={GOLD_OFFENSE.has(key) ? "gold" : undefined}
          />
        ))}
      </Section>

      <Section title="Defense">
        {DEFENSE_KEYS.map((key) => (
          <StatLine
            key={key}
            statKey={key}
            value={effectiveStatValue(stats, key)}
            highlight={
              GOLD_DEFENSE.has(key)
                ? "gold"
                : BLUE_DEFENSE.has(key)
                  ? "blue"
                  : undefined
            }
          />
        ))}
      </Section>

      <Section title="Resistances">
        {RESISTANCES.map((r) => {
          const v = stats[r.key] ?? 0;
          const cap = effectiveCap(r.key, stats);
          const zero = isZero(v);
          const numeric = typeof v === "number" ? v : 0;
          const capped = cap !== undefined && numeric > cap;
          return (
            <div
              key={r.key}
              className="flex items-baseline justify-between gap-2 py-0.75"
            >
              <span className={`${r.className} flex-1 min-w-0 leading-tight`}>
                {r.label}
              </span>
              <span
                className={`font-mono tabular-nums shrink-0 whitespace-nowrap text-right ${zero ? "text-faint" : r.className}`}
              >
                {zero ? (
                  "—"
                ) : capped ? (
                  <>
                    {cap}%{" "}
                    <span className="text-faint text-[10px]">({numeric}%)</span>
                  </>
                ) : (
                  formatValue(v, r.key)
                )}
              </span>
            </div>
          );
        })}
      </Section>
    </aside>
  );
}

function StatLine({
  statKey,
  value,
  highlight,
}: {
  statKey: string;
  value: RangedValue;
  highlight?: "gold" | "blue";
}) {
  const zero = isZero(value);
  const def = statDef(statKey);
  const label = def?.name ?? statKey;
  const labelClass = zero
    ? "text-faint"
    : highlight === "blue"
      ? "text-stat-blue"
      : "text-muted";
  const valueClass = zero
    ? "text-faint"
    : highlight === "gold"
      ? "text-accent-hot"
      : highlight === "blue"
        ? "text-stat-blue"
        : "text-text";
  return (
    <div className="flex items-baseline justify-between gap-2 py-0.75">
      <span className={`${labelClass} flex-1 min-w-0 leading-tight`}>
        {label}
      </span>
      <span
        className={`font-mono tabular-nums shrink-0 whitespace-nowrap text-right ${valueClass}`}
      >
        {zero ? "—" : formatValue(value, statKey)}
      </span>
    </div>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="border-b border-border/70 px-4 py-3">
      <div className="mb-2 flex items-center gap-2 border-b border-accent-deep/20 pb-1.5">
        <span
          aria-hidden
          className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
        />
        <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/70">
          {title}
        </span>
      </div>
      <div className="space-y-px">{children}</div>
    </div>
  );
}

function Row({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-2 py-0.75">
      <span className="text-muted flex-1 min-w-0 leading-tight">{label}</span>
      <span className="font-mono tabular-nums shrink-0 whitespace-nowrap text-right">
        {value}
      </span>
    </div>
  );
}

function PanelSelect({
  className,
  children,
  ...rest
}: React.SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <div
      className={`inline-flex w-full items-center rounded-[3px] border border-border-2 px-2 py-1.5 transition-colors hover:border-accent-deep focus-within:border-accent-hot ${className ?? ""}`}
      style={{
        background:
          "linear-gradient(180deg, #0d0e12, var(--color-panel-2))",
        boxShadow: "inset 0 1px 2px rgba(0,0,0,0.5)",
      }}
    >
      <select
        {...rest}
        className="w-full cursor-pointer bg-transparent text-[12px] text-text outline-none"
      >
        {children}
      </select>
    </div>
  );
}

function formatNumRange(min: number, max: number): string {
  const fmt = (v: number) =>
    Number.isInteger(v) ? String(v) : v.toFixed(2).replace(/\.?0+$/, "");
  if (Math.abs(min - max) < 0.005) return fmt(min);
  return `${fmt(min)}–${fmt(max)}`;
}
