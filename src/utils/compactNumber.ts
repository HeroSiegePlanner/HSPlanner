export type NumberScale = 'none' | 'thousands' | 'millions' | 'billions'

export const NUMBER_SCALES: NumberScale[] = [
  'none',
  'thousands',
  'millions',
  'billions',
]

// Largest divider each scale may apply; smaller units cascade below it.
const SCALE_CAP: Record<NumberScale, number> = {
  none: 0,
  thousands: 1e3,
  millions: 1e6,
  billions: 1e9,
}

function inUnit(n: number, unit: number, suffix: string): string {
  const value = (n / unit).toLocaleString(undefined, {
    maximumFractionDigits: 1,
  })
  return `${value}${suffix}`
}

export function compact(n: number, scale: NumberScale = 'billions'): string {
  const abs = Math.abs(n)
  const cap = SCALE_CAP[scale]
  if (abs >= 1e9 && cap >= 1e9) return inUnit(n, 1e9, 'B')
  if (abs >= 1e6 && cap >= 1e6) return inUnit(n, 1e6, 'M')
  if (abs >= 1e4 && cap >= 1e3) return inUnit(n, 1e3, 'k')
  return Math.round(n).toLocaleString()
}

export function compactRange(
  lo: number,
  hi: number,
  scale: NumberScale = 'billions',
): string {
  return Math.abs(lo - hi) < 0.5
    ? compact(lo, scale)
    : `${compact(lo, scale)}–${compact(hi, scale)}`
}
