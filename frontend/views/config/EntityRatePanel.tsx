import { useMemo } from 'react'
import { skills } from '@data'
import { useBuild } from '../../store/build'
import { effectiveSkillTags, entityTagOf } from '../../utils/skills/skillTags'
import {
  ENTITY_KINDS,
  DEFAULT_ENTITY_RATE,
  entityKindOfTag,
  type EntityKind,
} from '../../utils/build/entityRates'
import { Panel } from './configPrimitives'

const KIND_LABEL: Record<EntityKind, string> = {
  sentry: 'Sentry',
  summon: 'Summon',
  guardian: 'Guardian',
}

export default function EntityRatePanel() {
  const rates = useBuild((s) => s.entityRates)
  const setRate = useBuild((s) => s.setEntityRate)
  const skillRanks = useBuild((s) => s.skillRanks)
  const subskillRanks = useBuild((s) => s.subskillRanks)

  const kindsInBuild = useMemo(() => {
    const found = new Set<EntityKind>()
    for (const s of skills) {
      if ((skillRanks[s.id] ?? 0) === 0) continue
      const tag = entityTagOf(effectiveSkillTags(s, subskillRanks))
      const kind = tag ? entityKindOfTag(tag) : undefined
      if (kind) found.add(kind)
    }
    return ENTITY_KINDS.filter((k) => found.has(k))
  }, [skillRanks, subskillRanks])

  if (kindsInBuild.length === 0) return null

  return (
    <Panel
      title="Entity Attack Rate"
      subtitle="Base attacks/casts per second of the entities a skill fields. Sentries, summons and guardians are separate kinds with separate rates because the game exposes none of them, so tune each by hand."
    >
      <div className="space-y-2">
        {kindsInBuild.map((kind) => (
          <RateRow
            key={kind}
            label={KIND_LABEL[kind]}
            value={rates[kind]}
            onChange={(rate) => setRate(kind, rate)}
          />
        ))}
      </div>
    </Panel>
  )
}

function RateRow({
  label,
  value,
  onChange,
}: {
  label: string
  value: number
  onChange: (rate: number) => void
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
        {label} / sec
      </span>
      <div
        className="inline-flex w-20 shrink-0 items-center rounded-[3px] border border-border-2 px-2 py-1 transition-colors focus-within:border-accent-hot"
        style={{
          background: 'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
          boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
        }}
      >
        <input
          type="number"
          min={0}
          step={0.1}
          value={value}
          onChange={(e) => {
            const raw = e.target.value
            if (raw === '') {
              onChange(DEFAULT_ENTITY_RATE)
              return
            }
            const n = Number(raw)
            if (!Number.isFinite(n)) return
            onChange(Math.max(0, n))
          }}
          className="w-full bg-transparent text-right font-mono text-[12px] tabular-nums text-accent-hot outline-none"
        />
      </div>
    </div>
  )
}
