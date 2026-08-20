import type { PickerRow } from '../PickerModal'
import { gems, runes } from '@data'
import { fmtStats } from '../../../utils/item/stats'
import { gemColorForName, socketableIconForName } from './gearIcons'
import { buildSocketableTooltip } from '../tooltips'

let cache: PickerRow[] | null = null

export function getSocketPickerRows(): PickerRow[] {
  if (cache) return cache
  const out: PickerRow[] = []
  for (const g of gems) {
    const isJewel = g.name.toLowerCase().includes('jewel')
    const kind = isJewel ? 'JEWEL' : 'GEM'
    out.push({
      id: g.id,
      name: g.name,
      tier: g.tier,
      kindLabel: kind,
      group: isJewel ? 'Jewels' : 'Gems',
      meta: fmtStats(g.stats) || '—',
      iconColor: gemColorForName(g.name),
      iconUrl: socketableIconForName(g.name),
      tooltip: buildSocketableTooltip(g, kind),
    })
  }
  for (const r of runes) {
    out.push({
      id: r.id,
      name: r.name,
      tier: r.tier,
      kindLabel: 'RUNE',
      group: 'Runes',
      meta: fmtStats(r.stats) || '—',
      iconColor: 'var(--color-accent)',
      iconUrl: socketableIconForName(r.name),
      tooltip: buildSocketableTooltip(r, 'RUNE'),
    })
  }
  cache = out
  return out
}
