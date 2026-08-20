interface ProgressionSliderProps {
  total: number
  value: number | null
  onChange: (value: number | null) => void
  accentClass?: string
}

export default function ProgressionSlider({
  total,
  value,
  onChange,
  accentClass = 'text-accent-hot',
}: ProgressionSliderProps) {
  if (total < 2) return null
  const current = value ?? total
  return (
    <div
      className="absolute bottom-3.5 left-1/2 z-10 flex -translate-x-1/2 items-center gap-3 rounded-[3px] border border-border px-3 py-1.5"
      style={{
        background:
          'linear-gradient(180deg, color-mix(in srgb, var(--color-panel-2) 80%, transparent), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        backdropFilter: 'blur(6px)',
        boxShadow:
          'inset 0 1px 0 rgba(201,165,90,0.06), 0 4px 16px rgba(0,0,0,0.45)',
      }}
    >
      <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
        Progression
      </span>
      <input
        type="range"
        min={1}
        max={total}
        step={1}
        value={current}
        aria-label="Progression step"
        onChange={(e) => {
          const v = Number(e.target.value)
          onChange(v >= total ? null : v)
        }}
        className="w-[300px] max-w-[38vw]"
      />
      <span
        className={`whitespace-nowrap font-mono text-[10px] tracking-[0.14em] ${
          value == null ? 'text-faint' : accentClass
        }`}
      >
        {current} / {total}
      </span>
    </div>
  )
}
