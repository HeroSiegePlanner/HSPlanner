import { Fragment, useMemo } from "react";
import { gameConfig, getClass, getSkillsByClass } from "@data";
import { EhpRows } from "../EhpRows";
import { compactRange } from "../../utils/compactNumber";
import { useSettings } from "../../store/settings";
import {
  attrPointsFor,
  skillPointsFor,
  useBuild,
} from "../../store/build";
import {
  effectiveCap,
  formatValue,
  isZero,
  normalizeSkillName,
  rangedMax,
  rangedMin,
  statDef,
} from "../../utils/item/stats";
import { computeBuildPerformanceAsync } from "../../utils/calc/bridge";
import type { BuildPerformance } from "../../utils/build/buildPerformance";
import { heroLevelFor } from "../../utils/build/heroLevel";
import { useBuildPerformanceDeps } from "../../hooks/useBuildPerformanceDeps";
import { useCalcResult } from "../../hooks/useCalcResult";
import type { RangedValue } from "../../types";
import {
  ATTRIBUTE_ORDER,
  ATTR_COLOR,
  BLUE_DEFENSE,
  DEFENSE_KEYS,
  GOLD_DEFENSE,
  GOLD_OFFENSE,
  OFFENSE_KEYS,
  RESISTANCES,
  effectiveStatValue,
} from "../../utils/build/statSectionDefs";
import { computeSustainStats } from "../../utils/build/sustainStats";
import {
  effectiveSkillTags,
  entityTagOf,
} from "../../utils/skills/skillTags";
import {
  entityAttackRate,
  entityAttackRateFixedKey,
  entityAttackSpeedKey,
  entityKindOfTag,
} from "../../utils/build/entityRates";

export default function LeftStatsPanel() {
  const classId = useBuild((s) => s.classId);
  const level = useBuild((s) => s.level);
  const allocated = useBuild((s) => s.allocated);
  const skillRanks = useBuild((s) => s.skillRanks);
  const activeSkillIds = useBuild((s) => s.activeSkillIds);
  const subskillRanks = useBuild((s) => s.subskillRanks);
  const toggleActiveSkill = useBuild((s) => s.toggleActiveSkill);
  const numberScale = useSettings((s) => s.numberScale);

  const buildDeps = useBuildPerformanceDeps();
  const performance = useCalcResult<BuildPerformance | null>(
    () => computeBuildPerformanceAsync(buildDeps),
    [buildDeps],
    null,
  );
  const attributes = performance?.attributes ?? {};
  const stats = performance?.stats ?? {};
  const statsCombined = performance?.statsCombined ?? {};
  const diminishedRaw = performance?.diminishedRaw ?? {};
  const damage = performance?.damage ?? null;
  const attackDamage = performance?.attackDamage ?? null;
  const hitDpsMin = performance?.hitDpsMin;
  const hitDpsMax = performance?.hitDpsMax;
  const combinedDpsMin = performance?.combinedDpsMin;
  const combinedDpsMax = performance?.combinedDpsMax;
  const ailmentDpsMin = performance?.ailmentDpsMin;
  const ailmentDpsMax = performance?.ailmentDpsMax;

  const cls = classId ? getClass(classId) : undefined;
  const attrSpent = Object.values(allocated).reduce((s, v) => s + v, 0);
  const attrTotal = attrPointsFor(level);
  const skillSpent = Object.values(skillRanks).reduce((s, v) => s + v, 0);
  const skillTotal = skillPointsFor(level);
  const heroLevel = heroLevelFor(buildDeps);

  const allClassSkills = useMemo(() => getSkillsByClass(classId), [classId]);
  const classSkills = useMemo(
    () => allClassSkills.filter((s) => s.kind === "active"),
    [allClassSkills],
  );
  const primarySkillId = activeSkillIds[0] ?? null;
  const activeSkill =
    primarySkillId != null
      ? classSkills.find((s) => s.id === primarySkillId)
      : null;
  const activeRank = activeSkill ? (skillRanks[activeSkill.id] ?? 0) : 0;

  // Sentry/Summon/Guardian skills field several entities that swing on their
  // own cadence; the DPS rows already fold both in, so name them here instead
  // of leaving the multipliers implicit.
  const entityCount = performance?.entityCount;
  const entityLabel = activeSkill
    ? entityTagOf(effectiveSkillTags(activeSkill, subskillRanks))
    : undefined;
  const entityKind = entityLabel ? entityKindOfTag(entityLabel) : undefined;
  const entityRates = useBuild((s) => s.entityRates);
  const entitySwing = entityKind
    ? entityAttackRate(
        entityKind,
        entityRates,
        [
          rangedMin(stats[entityAttackSpeedKey(entityKind)] ?? 0),
          rangedMax(stats[entityAttackSpeedKey(entityKind)] ?? 0),
        ],
        rangedMax(stats[entityAttackRateFixedKey(entityKind)] ?? 0),
      )
    : undefined;

  const rankBonus: [number, number] = activeSkill
    ? (performance?.rankBonuses[normalizeSkillName(activeSkill.name)] ?? [0, 0])
    : [0, 0];
  const rankBonusMin = rankBonus[0];
  const rankBonusMax = rankBonus[1];
  const sustain = useCalcResult(
    () =>
      activeSkill
        ? computeSustainStats({
            skill: activeSkill,
            activeRank,
            rankBonusMin,
            rankBonusMax,
            stats,
            statsCombined,
          })
        : null,
    [activeSkill, activeRank, rankBonusMin, rankBonusMax, stats, statsCombined],
    null,
  );
  const effRankMin = sustain?.effRankMin ?? activeRank + rankBonusMin;
  const effRankMax = sustain?.effRankMax ?? activeRank + rankBonusMax;
  const effManaMin = sustain?.effManaMin;
  const effManaMax = sustain?.effManaMax;
  const lifePerCastMin = sustain?.lifePerCastMin;
  const lifePerCastMax = sustain?.lifePerCastMax;
  const effCastMin = sustain?.effCastMin;
  const effCastMax = sustain?.effCastMax;
  const manaPerSecMin = sustain?.manaPerSecMin;
  const manaPerSecMax = sustain?.manaPerSecMax;
  const manaRegenMin = sustain?.manaRegenMin ?? 0;
  const manaRegenMax = sustain?.manaRegenMax ?? 0;
  const sustainable = sustain?.sustainable ?? false;
  const unsustainable = sustain?.unsustainable ?? false;
  const netMin = sustain?.netMin;
  const netMax = sustain?.netMax;
  const uptimeMin = sustain?.uptimeMin;
  const uptimeMax = sustain?.uptimeMax;

  return (
    <aside
      data-tour="left-stats"
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
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
              <span
                aria-hidden
                className="inline-block h-1.5 w-1.5 shrink-0 rotate-45 bg-accent-hot"
                style={{ boxShadow: "0 0 8px rgba(224,184,100,0.6)" }}
              />
              <span>Character</span>
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
          <span className="flex shrink-0 flex-col items-end font-mono text-[10px] uppercase leading-tight tracking-[0.18em] text-accent-hot">
            <span>Lv {level}</span>
            <span>Hero Lv {heroLevel}</span>
          </span>
        </div>
      </div>

      <Section title="Active Skills">
        {classSkills.length === 0 ? (
          <div className="font-mono text-[11px] tracking-[0.04em] text-muted italic">
            No skills for this class
          </div>
        ) : (
          <>
            {activeSkillIds.length === 0 ? (
              <div className="mb-2 font-mono text-[11px] tracking-[0.04em] text-muted italic">
                Pick active skills in the Skills tab
              </div>
            ) : (
              <div className="mb-2 flex flex-col gap-1">
                {activeSkillIds.map((id) => {
                  const sk = classSkills.find((s) => s.id === id);
                  const ps = performance?.perSkill?.find((p) => p.id === id);
                  const dps =
                    ps?.hitDpsMin !== undefined && ps?.hitDpsMax !== undefined
                      ? compactRange(ps.hitDpsMin, ps.hitDpsMax, numberScale)
                      : "—";
                  return (
                    <button
                      key={id}
                      onClick={() => toggleActiveSkill(id)}
                      title={`Remove ${sk?.name ?? id} from active skills`}
                      className="flex items-center justify-between gap-2 rounded-[3px] border border-border-2 px-2 py-1 text-left transition-colors hover:border-stat-red/60"
                      style={{ background: "var(--color-panel-2)" }}
                    >
                      <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-text">
                        {sk?.name ?? id}
                      </span>
                      <span className="shrink-0 font-mono text-[11px] tabular-nums text-accent-hot">
                        {dps}
                      </span>
                    </button>
                  );
                })}
              </div>
            )}
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
                {lifePerCastMax !== undefined && lifePerCastMax > 0 && (
                  <Row
                    label="Life / cast"
                    value={
                      <span className="text-stat-red">
                        {formatNumRange(lifePerCastMin ?? 0, lifePerCastMax)}
                      </span>
                    }
                  />
                )}
                {entitySwing && (
                  <Row
                    label="Attack rate"
                    value={
                      <span className="text-text">
                        {formatNumRange(entitySwing.min, entitySwing.max)}/s
                      </span>
                    }
                  />
                )}
                <Row
                  label={
                    entityKind
                      ? 'Spawn rate'
                      : activeSkill?.usesAttackSpeed
                        ? 'Attack rate'
                        : 'Cast rate'
                  }
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
                        {compactRange(damage.finalMin, damage.finalMax, numberScale)}
                      </span>
                    ) : attackDamage ? (
                      <span className="text-text">
                        {compactRange(
                          attackDamage.combinedHitMin,
                          attackDamage.combinedHitMax,
                          numberScale,
                        )}
                      </span>
                    ) : (
                      <span className="text-muted">—</span>
                    )
                  }
                />
                {entityCount && entityLabel && (
                  <Row
                    label={`${entityLabel} count`}
                    value={
                      <span className="text-accent-hot">
                        ×{formatNumRange(entityCount[0], entityCount[1])}
                      </span>
                    }
                  />
                )}
                <Row
                  label="Hit DPS"
                  value={
                    hitDpsMin !== undefined && hitDpsMax !== undefined ? (
                      <span className="text-accent-hot">
                        {compactRange(hitDpsMin, hitDpsMax, numberScale)}
                      </span>
                    ) : (
                      <span className="text-muted">—</span>
                    )
                  }
                />
                {ailmentDpsMin !== undefined &&
                  ailmentDpsMax !== undefined && (
                    <Row
                      label="Ailment DPS"
                      value={
                        <span className="text-accent-hot">
                          {compactRange(
                            ailmentDpsMin,
                            ailmentDpsMax,
                            numberScale,
                          )}
                        </span>
                      }
                    />
                  )}
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
                        {compactRange(combinedDpsMin, combinedDpsMax, numberScale)}
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
          value={
            <span className="text-text">
              {heroLevel}
            </span>
          }
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
            value={effectiveStatValue(stats, statsCombined, key)}
            rawValue={diminishedRaw[key]}
            highlight={GOLD_OFFENSE.has(key) ? "gold" : undefined}
          />
        ))}
      </Section>

      <Section title="Defense">
        {DEFENSE_KEYS.map((key) => (
          <Fragment key={key}>
            <StatLine
              statKey={key}
              value={effectiveStatValue(stats, statsCombined, key)}
              rawValue={diminishedRaw[key]}
              highlight={
                GOLD_DEFENSE.has(key)
                  ? "gold"
                  : BLUE_DEFENSE.has(key)
                    ? "blue"
                    : undefined
              }
            />
            {key === "mana_replenish" && (
              <EhpRows stats={stats} statsCombined={statsCombined} />
            )}
          </Fragment>
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
  rawValue,
  highlight,
}: {
  statKey: string;
  value: RangedValue;
  /** Pre-diminishing-returns total; shown muted next to the effective value. */
  rawValue?: RangedValue;
  highlight?: "gold" | "blue";
}) {
  const zero = isZero(value);
  const def = statDef(statKey);
  const label = def?.name ?? statKey;
  const labelClass =
    highlight === "blue" ? "text-stat-blue" : "text-muted";
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
        {!zero && rawValue !== undefined && (
          <span className="block text-faint font-normal text-[10px] leading-tight">
            ({formatValue(rawValue, statKey)})
          </span>
        )}
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

function formatNumRange(min: number, max: number): string {
  const fmt = (v: number) =>
    Number.isInteger(v) ? String(v) : v.toFixed(2).replace(/\.?0+$/, "");
  if (Math.abs(min - max) < 0.005) return fmt(min);
  return `${fmt(min)}–${fmt(max)}`;
}
