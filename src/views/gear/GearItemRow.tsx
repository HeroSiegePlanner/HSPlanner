import { getItem } from '../../data'
import { ItemTooltipBody, RARITY_TONE } from '../../components/ItemTooltip'
import Tooltip from '../../components/Tooltip'
import type { PickerRow } from '../../components/PickerModal'
import { gemTintForRarity } from './lib/icons'
import { RARITY_TEXT } from './lib/rarity'

export function GearItemRow({
  row,
  selected,
  onSelect,
  onHover,
}: {
  row: PickerRow
  selected: boolean
  onSelect: () => void
  onHover: () => void
}) {
  const rarity = row.rarity
  const nameColor = rarity ? RARITY_TEXT[rarity] : 'text-text'

  const itemBase = getItem(row.id)
  const tooltipTone = itemBase ? RARITY_TONE[itemBase.rarity] : 'neutral'

  const button = (
    <button
      type="button"
      onClick={onSelect}
      onMouseEnter={onHover}
      className={`group relative grid w-full cursor-pointer items-center gap-3 border-b border-border px-4 py-2.5 text-left transition-colors last:border-b-0 hover:bg-accent-hot/5 ${
        selected ? 'bg-accent-hot/5' : ''
      }`}
      style={{ gridTemplateColumns: '40px 1fr auto' }}
    >
      <span
        className={`pointer-events-none absolute left-0 top-0 bottom-0 w-0.5 bg-accent transition-opacity ${
          selected ? 'opacity-100' : 'opacity-0 group-hover:opacity-50'
        }`}
      />
      <span className="flex h-9 w-9 items-center justify-center">
        {row.iconUrl ? (
          <img
            src={row.iconUrl}
            alt=""
            className="h-full w-full object-contain"
            style={{ imageRendering: 'pixelated' }}
          />
        ) : (
          <span
            className="block h-7 w-7 rotate-45 rounded-xs"
            style={{
              background: `linear-gradient(135deg, ${gemTintForRarity(rarity)}, #0d0b07)`,
              border: `1px solid color-mix(in srgb, ${gemTintForRarity(rarity)} 60%, #000)`,
            }}
            aria-hidden="true"
          />
        )}
      </span>
      <span className={`truncate text-[12px] font-medium ${nameColor}`}>
        {row.name}
      </span>
      <span className="truncate font-mono text-[9px] tracking-[0.04em] text-muted/80 max-w-45">
        {typeof row.meta === 'string' ? row.meta : ''}
      </span>
    </button>
  )

  if (!itemBase) return button

  return (
    <Tooltip
      content={<ItemTooltipBody base={itemBase} />}
      tone={tooltipTone}
      placement="right"
      delay={150}
    >
      {button}
    </Tooltip>
  )
}
