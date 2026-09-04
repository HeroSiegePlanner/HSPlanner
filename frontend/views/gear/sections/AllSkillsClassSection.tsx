import { useMemo } from 'react'
import { classes } from '@data'
import type { EquippedItem } from '../../../types'
import Dropdown from '../../../components/ui/Dropdown'
import { SectionCard } from '../SectionCard'
import { SectionIcon } from '../sectionIcons'

export function AllSkillsClassSection({
  equipped,
  onChange,
}: {
  equipped: EquippedItem
  onChange: (classId: string | null) => void
}) {
  const options = useMemo(
    () =>
      classes
        .map((c) => ({ id: c.id, label: c.name }))
        .sort((a, b) => a.label.localeCompare(b.label)),
    [],
  )

  const picked = equipped.allSkillsClassId ?? null

  return (
    <SectionCard
      label="All Skills Class"
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
        placeholder="Pick the rolled class"
        searchPlaceholder="Search class…"
        clearLabel="No class"
      />
      <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] leading-snug text-faint">
        The roll names one class. It only adds skill ranks on a build of that
        class.
      </p>
    </SectionCard>
  )
}
