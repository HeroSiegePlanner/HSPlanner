import { useMemo } from 'react'
import { getAffix, getGem, getRune } from '../../data'
import { formatValue, statName } from '../../utils/item/stats'
import { useAffixDisplayRanges } from '../gear/sections/AffixesSection'
import type { TreeSocketContent } from '../../types'
import {
  TooltipSection,
  TooltipSectionHeader,
  TooltipText,
} from '../../components/Tooltip'

export function JewelrySocketSection({
  content,
  isAllocated,
}: {
  content: TreeSocketContent | null
  isAllocated: boolean
}) {
  const craftedAffixes = useMemo(
    () => (content && content.kind !== 'item' ? content.affixes : []),
    [content],
  )
  const craftedItems = useMemo(
    () =>
      craftedAffixes.map((eq) => ({
        def: getAffix(eq.affixId),
        roll: eq.roll,
      })),
    [craftedAffixes],
  )
  const craftedValues = useAffixDisplayRanges(craftedItems)

  if (!content) {
    return (
      <TooltipSection>
        <TooltipSectionHeader tone="gold">Socketed</TooltipSectionHeader>
        <TooltipText>
          <span className="text-faint italic">
            Empty socket{isAllocated ? ' — right-click to insert' : ''}
          </span>
        </TooltipText>
      </TooltipSection>
    )
  }

  let socketedTitle: string
  let socketedSubtitle: string | null = null
  let statLines: { key: string; value: number }[] = []

  if (content.kind === 'item') {
    const source = getGem(content.id) ?? getRune(content.id)
    if (!source) {
      return (
        <TooltipSection>
          <TooltipSectionHeader tone="gold">Socketed</TooltipSectionHeader>
          <TooltipText>
            <span className="text-stat-red">
              unknown socketable: {content.id}
            </span>
          </TooltipText>
        </TooltipSection>
      )
    }
    socketedTitle = source.name
    socketedSubtitle = `T${source.tier}`
    statLines = Object.entries(source.stats)
      .filter(([, v]) => v !== 0)
      .map(([key, value]) => ({ key, value }))
  } else {
    socketedTitle = 'Uncut Jewel'
    socketedSubtitle = `${content.affixes.length} affix${
      content.affixes.length === 1 ? '' : 'es'
    }`
    statLines = craftedAffixes
      .map((eq, idx) => {
        const def = getAffix(eq.affixId)
        if (!def || !def.statKey) return null
        const value = craftedValues[idx]?.value ?? 0
        if (value === 0) return null
        return { key: def.statKey, value }
      })
      .filter((x): x is { key: string; value: number } => x !== null)
  }

  return (
    <>
      <TooltipSection>
        <TooltipSectionHeader tone="gold" trailing={socketedSubtitle}>
          Socketed
        </TooltipSectionHeader>
        <div className="text-[12px] font-medium text-accent-hot">
          {socketedTitle}
        </div>
      </TooltipSection>
      {statLines.length > 0 && (
        <TooltipSection>
          <TooltipSectionHeader tone="gold">From Sockets</TooltipSectionHeader>
          <ul className="space-y-0.5 text-[12px]">
            {statLines.map(({ key, value }) => (
              <li key={key} className="text-accent">
                {formatValue(value, key)} {statName(key)}
              </li>
            ))}
          </ul>
        </TooltipSection>
      )}
    </>
  )
}
