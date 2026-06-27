export function compact(n: number): string {
  const abs = Math.abs(n)
  const strip = (s: string) => s.replace(/\.0$/, '')
  if (abs >= 1e9) return `${strip((n / 1e9).toFixed(1))}B`
  if (abs >= 1e6) return `${strip((n / 1e6).toFixed(1))}M`
  if (abs >= 1e4) return `${strip((n / 1e3).toFixed(1))}k`
  return Math.round(n).toLocaleString()
}

export function compactRange(lo: number, hi: number): string {
  return Math.abs(lo - hi) < 0.5 ? compact(lo) : `${compact(lo)}–${compact(hi)}`
}
