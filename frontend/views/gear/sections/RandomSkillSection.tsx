import { useMemo } from 'react'
import { skills } from '@data'
import type { EquippedItem, RandomSkillPool } from '../../../types'
import Dropdown from '../../../components/ui/Dropdown'
import { SectionCard } from '../SectionCard'
import { SectionIcon } from '../sectionIcons'

export function RandomSkillSection({
  equipped,
  pool,
  onChange,
}: {
  equipped: EquippedItem
  pool: RandomSkillPool
  onChange: (skillId: string | null) => void
}) {
  const options = useMemo(
    () =>
      skills
        .filter((s) => s.classId === pool.classId && s.tree === pool.tree)
        .map((s) => ({ id: s.id, label: s.name }))
        .sort((a, b) => a.label.localeCompare(b.label)),
    [pool.classId, pool.tree],
  )

  const picked = equipped.randomSkillId ?? null

  return (
    <SectionCard
      label="Random Skill"
      icon={<SectionIcon kind="randomSkill" />}
      collapsible
      defaultOpen={picked != null}
      rightSlot={
        <span
          className={`max-w-[200px] truncate font-mono text-[10px] tracking-[0.04em] ${
            picked ? 'text-accent-hot' : 'text-faint'
          }`}
        >
          {picked
            ? (options.find((o) => o.id === picked)?.label ?? 'rolled')
            : 'not rolled'}
        </span>
      }
    >
      <Dropdown
        value={picked}
        options={options}
        onChange={onChange}
        placeholder="Pick the rolled skill"
        searchPlaceholder="Search skill…"
        clearLabel="No skill"
      />
      <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] leading-snug text-faint">
        The roll is random in game — pick the one yours landed on to count its
        ranks.
      </p>
    </SectionCard>
  )
}
