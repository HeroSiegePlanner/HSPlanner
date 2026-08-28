import { SKILL_ELEMENTS } from '../../../types'
import type { EquippedItem, SkillElement } from '../../../types'
import Dropdown from '../../../components/ui/Dropdown'
import { SectionCard } from '../SectionCard'
import { SectionIcon } from '../sectionIcons'

const OPTIONS = SKILL_ELEMENTS.map((e) => ({
  id: e,
  label: `${e.charAt(0).toUpperCase()}${e.slice(1)} Skills`,
}))

export function RandomElementSection({
  equipped,
  onChange,
}: {
  equipped: EquippedItem
  onChange: (element: SkillElement | null) => void
}) {
  const picked = equipped.randomSkillElement ?? null

  return (
    <SectionCard
      label="Random Skill Element"
      icon={<SectionIcon kind="element" />}
      collapsible
      defaultOpen={picked != null}
      rightSlot={
        <span
          className={`font-mono text-[10px] uppercase tracking-[0.04em] ${
            picked ? 'text-accent-hot' : 'text-faint'
          }`}
        >
          {picked ?? 'not rolled'}
        </span>
      }
    >
      <Dropdown
        value={picked}
        options={OPTIONS}
        onChange={(id) => onChange(id as SkillElement | null)}
        placeholder="Pick the rolled element"
        clearLabel="No element"
        searchable={false}
      />
      <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] leading-snug text-faint">
        The element is random in game — pick the one yours landed on to count
        its skill ranks.
      </p>
    </SectionCard>
  )
}
