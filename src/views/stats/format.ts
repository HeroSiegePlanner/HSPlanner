import { useSettings } from '../../store/settings'
import { compact, type NumberScale } from '../../utils/compactNumber'

export function formatDecimal(v: number): string {
  if (Number.isInteger(v)) return String(v)
  return v.toFixed(2)
}

export function formatRange(min: number, max: number): string {
  if (min === max) return formatDecimal(min)
  return `${formatDecimal(min)}-${formatDecimal(max)}`
}

export function formatRangeInt(min: number, max: number): string {
  if (min === max) return String(min)
  return `${min}-${max}`
}

export function displayRange(min: number, max: number): string {
  if (min === max) return String(min)
  return `${min}–${max}`
}

export function scaledRangeInt(
  min: number,
  max: number,
  scale: NumberScale,
): string {
  if (min === max) return compact(min, scale)
  return `${compact(min, scale)}-${compact(max, scale)}`
}

export function useFormatRangeInt(): (min: number, max: number) => string {
  const scale = useSettings((s) => s.numberScale)
  return (min, max) => scaledRangeInt(min, max, scale)
}
