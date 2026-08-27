export function sliderStep(a: number, b: number): number {
  return Number.isInteger(a) && Number.isInteger(b) ? 1 : 0.1
}

// Engine maps roll 0..1 onto the |min|..|max| magnitude; sign lives on the endpoints.
export function rollFromValue(value: number, rangeA: number, rangeB: number): number {
  const lo = Math.min(Math.abs(rangeA), Math.abs(rangeB))
  const hi = Math.max(Math.abs(rangeA), Math.abs(rangeB))
  if (hi === lo) return 1
  const roll = (Math.abs(value) - lo) / (hi - lo)
  return Math.max(0, Math.min(1, roll))
}

// Local mirror of the engine's rolled value so dragging never waits on IPC.
export function valueFromRoll(
  roll: number,
  rangeA: number,
  rangeB: number,
  format: 'flat' | 'percent',
): number {
  const lo = Math.min(Math.abs(rangeA), Math.abs(rangeB))
  const hi = Math.max(Math.abs(rangeA), Math.abs(rangeB))
  const clamped = Math.max(0, Math.min(1, roll))
  const mag = lo + (hi - lo) * clamped
  const rounded = format === 'flat' ? Math.round(mag) : mag
  return rangeA < 0 || rangeB < 0 ? -rounded : rounded
}

export function formatAffixValue(
  affix: { sign: '+' | '-'; format: 'flat' | 'percent' },
  value: number,
): string {
  const abs = Math.abs(value)
  const n = Number.isInteger(abs) ? abs : Math.round(abs * 100) / 100
  const sign = value < 0 || affix.sign === '-' ? '-' : '+'
  return `${sign}${n}${affix.format === 'percent' ? '%' : ''}`
}

export function sliderPct(value: number, min: number, max: number): string {
  if (max === min) return '100%'
  const pct = ((value - min) / (max - min)) * 100
  return `${Math.max(0, Math.min(100, pct))}%`
}
