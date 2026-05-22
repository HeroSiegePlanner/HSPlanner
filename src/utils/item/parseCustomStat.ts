import type { RangedValue } from '../../types'

export function parseCustomStatValue(raw: string): RangedValue | null {
  // Parses a free-text user-entered stat value (such as "100", "+12", "12-18", "[12-18]", "100%") into a RangedValue (number or [min, max] tuple), returning null if the input cannot be parsed. Used by the custom-stat input UI to translate user text into the numeric form consumed by the stat-aggregation pipeline.
  const trimmed = raw.trim().replace(/[[\]]/g, '').replace(/%/g, '')
  if (!trimmed) return null

  const rangeMatch = trimmed.match(
    /^([+-]?\d+(?:\.\d+)?)\s*-\s*([+-]?\d+(?:\.\d+)?)$/,
  )
  if (rangeMatch) {
    const min = Number(rangeMatch[1])
    const max = Number(rangeMatch[2])
    if (!Number.isFinite(min) || !Number.isFinite(max)) return null
    return min === max ? min : [min, max]
  }

  const num = Number(trimmed)
  return Number.isFinite(num) ? num : null
}
