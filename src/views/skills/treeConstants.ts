import { DAMAGE_COLORS } from '../../utils/damageColors'
import type { DamageType, Skill } from '../../types'

export const CELL = 84
export const GAP = 18
export const ACCENT_HOT_RGB = '224,184,100'
export const SYNERGY_RGB = '167,139,250'

export function treeColorRgb(list: Skill[]): string {
  const counts = new Map<DamageType, number>()
  for (const s of list) {
    if (!s.damageType) continue
    counts.set(s.damageType, (counts.get(s.damageType) ?? 0) + 1)
  }
  let best: DamageType | null = null
  let bestN = 0
  for (const [t, n] of counts) {
    if (n > bestN) {
      best = t
      bestN = n
    }
  }
  return best ? DAMAGE_COLORS[best].rgb : ACCENT_HOT_RGB
}
