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
