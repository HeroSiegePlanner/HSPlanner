import { MAX_STARS } from '../../../store/build'
import { SectionCard } from '../SectionCard'
import { SectionIcon } from '../sectionIcons'

export function StarsSection({
  stars,
  onChange,
}: {
  stars: number
  onChange: (n: number) => void
}) {
  return (
    <SectionCard
      label="Stars"
      icon={<SectionIcon kind="stars" />}
      collapsible
      defaultOpen={stars > 0}
      rightSlot={
        stars > 0 ? (
          <>
            <span className="font-mono text-[10px] tabular-nums tracking-[0.04em] text-accent-hot">
              {'★'.repeat(stars)}
            </span>
            <button
              type="button"
              onClick={() => onChange(0)}
              className="rounded-xs border border-border-2 px-2 py-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-faint transition-colors hover:border-stat-red hover:text-stat-red"
            >
              Clear
            </button>
          </>
        ) : (
          <span className="font-mono text-[10px] tabular-nums tracking-[0.04em] text-faint">
            no bonus
          </span>
        )
      }
    >
      <div className="flex items-center gap-1.5">
        {Array.from({ length: MAX_STARS }).map((_, i) => {
          const target = i + 1
          const filled = target <= stars
          return (
            <button
              key={i}
              type="button"
              onClick={() => onChange(stars === target ? target - 1 : target)}
              aria-label={`${target} star${target === 1 ? '' : 's'}`}
              className={`text-[20px] leading-none transition-all ${
                filled
                  ? 'text-accent-hot hover:text-[#fff0c4]'
                  : 'text-muted/30 hover:text-accent-hot/50'
              }`}
              style={
                filled
                  ? {
                      textShadow:
                        '0 0 10px rgba(224,184,100,0.45), 0 0 2px rgba(224,184,100,0.6)',
                    }
                  : undefined
              }
            >
              ★
            </button>
          )
        })}
        {stars > 0 && (
          <span className="ml-1.5 font-mono text-[10px] tabular-nums text-faint">
            {stars}/{MAX_STARS}
          </span>
        )}
      </div>
      <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] leading-snug text-faint">
        Each star scales a stat by the game's own step for that stat. Many stats
        never scale, runeword items never do.
      </p>
    </SectionCard>
  )
}
