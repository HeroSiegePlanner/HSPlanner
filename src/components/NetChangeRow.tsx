import type { BuildStatDiff } from '../utils/build/buildPerformance'

export default function NetChangeRow({ diff }: { diff: BuildStatDiff }) {
  const fmtScalar = (v: number) => {
    if (diff.isPercent) {
      const rounded =
        Math.abs(v - Math.round(v)) < 0.05 ? Math.round(v) : Math.round(v * 10) / 10
      return `${rounded}%`
    }
    const abs = Math.abs(v)
    if (abs >= 1000) {
      return `${(v / 1000).toFixed(abs >= 10000 ? 1 : 2)}k`
    }
    const rounded =
      Math.abs(v - Math.round(v)) < 0.05 ? Math.round(v) : Math.round(v * 10) / 10
    return `${rounded}`
  }
  const fmtRange = (min: number, max: number) => {
    if (Math.abs(max - min) < 0.001) return fmtScalar(min)
    return `${fmtScalar(min)}-${fmtScalar(max)}`
  }
  const fmtDelta = (v: number) => {
    const sign = v > 0 ? '+' : ''
    return `${sign}${fmtScalar(v)}`
  }
  const tone =
    diff.kind === 'up'
      ? 'border-stat-green/35 bg-stat-green/8 text-stat-green'
      : 'border-stat-red/35 bg-stat-red/8 text-stat-red'
  const afterColor = diff.kind === 'up' ? 'text-stat-green' : 'text-stat-red'
  return (
    <div
      className="grid items-center gap-2 px-1 py-0.5 font-mono text-[11px] break-inside-avoid"
      style={{ gridTemplateColumns: '1fr auto auto auto auto' }}
    >
      <span className="font-sans text-[12px] text-text/85">{diff.label}</span>
      <span className="text-right tabular-nums text-faint">
        {fmtRange(diff.beforeMin, diff.beforeMax)}
      </span>
      <span className="text-faint">→</span>
      <span className={`text-right font-semibold tabular-nums ${afterColor}`}>
        {fmtRange(diff.afterMin, diff.afterMax)}
      </span>
      <span
        className={`min-w-[52px] rounded-[2px] border px-1.5 py-0.5 text-right text-[10px] font-semibold tabular-nums ${tone}`}
      >
        {fmtDelta(diff.delta)}
      </span>
    </div>
  )
}
