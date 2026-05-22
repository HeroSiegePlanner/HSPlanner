import ItemTooltip from '../../components/ItemTooltip'
import { CornerMarks } from '../../components/CornerMarks'
import { detectRuneword, getItem } from '../../data'
import type { EquippedItem, SlotKey } from '../../types'
import { RARITY_BG, RARITY_BORDER, RARITY_TEXT } from './lib/rarity'

export function SlotRow({
  slot,
  equipped,
  active,
  locked,
  onSelect,
}: {
  slot: { key: SlotKey; name: string }
  equipped: EquippedItem | undefined
  active: boolean
  locked?: boolean
  onSelect: () => void
}) {
  // Renders a single inventory-slot row in GearView's left rail with the slot label, the equipped item's name + rarity colour (or empty hint), the runeword override colour, and the click-to-select handler. Used once per slot inside GearView.
  const base = equipped ? getItem(equipped.baseId) : undefined
  const runeword =
    base && equipped ? detectRuneword(base, equipped.socketed) : undefined

  const rarityText = base
    ? runeword
      ? 'text-accent-hot'
      : RARITY_TEXT[base.rarity]
    : locked
      ? 'text-faint/50'
      : 'text-faint'
  const rarityBg = base
    ? RARITY_BG[base.rarity]
    : locked
      ? 'bg-panel-2/50'
      : 'bg-transparent'
  const rarityBorder = base
    ? RARITY_BORDER[base.rarity]
    : locked
      ? 'border-border/40 border-dashed'
      : 'border-border border-dashed'

  const badges: string[] = []
  if (base && equipped) {
    if (base.defenseMin !== undefined && base.defenseMax !== undefined)
      badges.push(`Def ${base.defenseMin}–${base.defenseMax}`)
    if (base.damageMin !== undefined && base.damageMax !== undefined)
      badges.push(`Dmg ${base.damageMin}–${base.damageMax}`)
    if (equipped.socketCount > 0)
      badges.push(
        `${equipped.socketed.filter(Boolean).length}/${equipped.socketCount}◇`,
      )
    if (equipped.stars && equipped.stars > 0)
      badges.push(`${'★'.repeat(equipped.stars)}`)
    if (base.requiresLevel) badges.push(`L${base.requiresLevel}`)
  }

  const button = (
    <button
      type="button"
      onClick={onSelect}
      className={`group flex h-full min-h-11 w-full items-center gap-2 rounded-[3px] border px-2 py-1.5 text-left transition-colors ${rarityBorder} ${
        active
          ? 'border-accent-hot bg-accent-hot/10 ring-1 ring-accent-hot/40'
          : `${rarityBg} hover:border-accent-deep`
      }`}
    >
      <span className="w-20 shrink-0 truncate font-mono text-[9px] font-semibold uppercase tracking-[0.18em] text-faint">
        {slot.name}
      </span>
      <span className="flex-1 min-w-0">
        {base ? (
          <>
            <span
              className={`block truncate text-[12px] font-semibold ${rarityText}`}
            >
              {runeword ? runeword.name : base.name}
            </span>
            <span className="block truncate text-[10px] text-muted">
              {runeword ? `Runeword · ${base.baseType}` : base.baseType}
            </span>
          </>
        ) : locked ? (
          <span className="block text-[11px] text-faint/60 italic">
            locked · 2H weapon equipped
          </span>
        ) : (
          <span className="block text-[11px] text-faint italic">empty</span>
        )}
      </span>
      {badges.length > 0 && (
        <span className="shrink-0 font-mono text-[9px] uppercase tracking-[0.14em] tabular-nums text-faint">
          {badges.join(' · ')}
        </span>
      )}
    </button>
  )

  if (!equipped) return button
  return (
    <ItemTooltip equipped={equipped} placement="right">
      {button}
    </ItemTooltip>
  )
}

export function GearPanel({
  title,
  trailing,
  children,
}: {
  title: string
  trailing?: React.ReactNode
  children: React.ReactNode
}) {
  // Renders the panel-system frame (gradient, accent corners, sectionLabel header) shared by Equipment and Charm Inventory in GearView.
  return (
    <section
      className="relative overflow-hidden rounded-md border border-border p-4"
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow:
          'inset 0 1px 0 rgba(255,255,255,0.02), 0 8px 24px rgba(0,0,0,0.35)',
      }}
    >
      <CornerMarks size={8} opacity={0.45} />
      <div className="mb-3 flex items-center justify-between gap-3 border-b border-accent-deep/20 pb-2">
        <div className="flex items-center gap-2">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 6px rgba(224,184,100,0.5)' }}
          />
          <h3 className="m-0 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-accent-hot/70">
            {title}
          </h3>
        </div>
        {trailing}
      </div>
      {children}
    </section>
  )
}

