import { formatValue } from '../../utils/item/stats'

export function formatPair(pair: [number, number]): string {
  return pair[0] === pair[1] ? String(pair[0]) : `${pair[0]}-${pair[1]}`
}

export function formatStatPair(key: string, min: number, max: number): string {
  if (min === max) return formatValue(min, key)
  return `${formatValue(min, key)}-${formatValue(max, key).replace(/^[+-]/, '')}`
}

export function evalFormulaClamped(
  f: { base: number; perLevel: number },
  rank: number,
): number {
  return Math.max(0, f.base + f.perLevel * rank)
}

export function formatPctRange(min: number, max: number): string {
  const m = Math.round(min * 100) / 100
  const mx = Math.round(max * 100) / 100
  if (m === mx) return `${m}%`
  return `${m}% - ${mx}%`
}

export function formatFlatPhys(
  minF: { base: number; perLevel: number },
  maxF: { base: number; perLevel: number },
  curMin: number,
  curMax: number,
): string {
  const fmt = (rank: number) =>
    `[${Math.round(evalFormulaClamped(minF, rank) * 100) / 100} to ${
      Math.round(evalFormulaClamped(maxF, rank) * 100) / 100
    }]`
  if (curMin === curMax) return fmt(curMin)
  return `${fmt(curMin)} … ${fmt(curMax)}`
}

export function formatDmgRange(
  min: [number, number],
  max: [number, number],
): string {
  if (min[0] === max[0] && min[1] === max[1]) return formatRangeTuple(min)
  return `${formatRangeTuple(min)} … ${formatRangeTuple(max)}`
}

export function formatRangeTuple([min, max]: [number, number]): string {
  const m = Math.round(min * 100) / 100
  const mx = Math.round(max * 100) / 100
  if (m === mx) return String(m)
  return `${m}-${mx}`
}
