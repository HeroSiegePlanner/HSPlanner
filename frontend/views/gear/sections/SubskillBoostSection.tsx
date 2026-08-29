import { useMemo } from 'react'
import { skills } from '@data'
import { useBuild } from '../../../store/build'
import type { EquippedItem } from '../../../types'
import Dropdown from '../../../components/ui/Dropdown'
import { SectionCard } from '../SectionCard'
import { SectionIcon } from '../sectionIcons'

export function SubskillBoostSection({
  equipped,
  onChange,
}: {
  equipped: EquippedItem
  onChange: (skillId: string | null) => void
}) {
  const classId = useBuild((s) => s.classId)

  const options = useMemo(
    () =>
      skills
        .filter((s) => s.subskills?.length && (!classId || s.classId === classId))
        .map((s) => ({ id: s.id, label: s.name }))
        .sort((a, b) => a.label.localeCompare(b.label)),
    [classId],
  )

  const picked = equipped.subskillBoostSkillId ?? null

  return (
    <SectionCard
      label="Random Skill Sub Skills"
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
        Raises every sub-skill you already put points into on that skill.
        Untouched nodes stay at zero.
      </p>
    </SectionCard>
  )
}
