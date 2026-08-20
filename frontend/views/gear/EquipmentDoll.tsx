import ItemTooltip from '../../components/ItemTooltip'
import { activeSeasonId, detectRuneword, effectiveStars, gameConfig, getItem, getItemImage } from '@data'
import { useBuild } from '../../store/build'
import type { EquippedItem, SlotKey } from '../../types'
import { RARITY_BG, RARITY_BORDER, RARITY_TEXT } from './lib/rarity'
import { GearPanel } from './SlotRail'

const RELIC_SLOTS: SlotKey[] = ['relic_1', 'relic_2', 'relic_3', 'relic_4', 'relic_5']
const POTION_SLOTS: SlotKey[] = ['potion_1', 'potion_2', 'potion_3', 'potion_4']

function slotName(key: SlotKey): string {
  return gameConfig.slots.find((s) => s.key === key)?.name ?? key
}

function SlotCell({
  slotKey,
  equipped,
  active,
  locked,
  onSelect,
  sizeClass,
  emptyLabel,
}: {
  slotKey: SlotKey
  equipped: EquippedItem | undefined
  active: boolean
  locked?: boolean
  onSelect: (s: SlotKey) => void
  sizeClass: string
  emptyLabel?: string
}) {
  const base = equipped ? getItem(equipped.baseId) : undefined
  const runeword = base && equipped ? detectRuneword(base, equipped.socketed) : undefined
  const sprite = base ? getItemImage(base.id) : undefined
  const name = slotName(slotKey)

  const border = base
    ? runeword
      ? 'border-accent-hot'
      : RARITY_BORDER[base.rarity]
    : locked
      ? 'border-dashed border-border/40'
      : 'border-dashed border-[#5a4528]'
  const bg = base ? RARITY_BG[base.rarity] : locked ? 'bg-panel-2/40' : 'bg-[#120c08]'

  const stars = equipped
    ? (effectiveStars(slotKey, activeSeasonId, equipped.stars) ?? 0)
    : 0
  const socketsFilled = equipped ? equipped.socketed.filter(Boolean).length : 0

  const button = (
    <button
      type="button"
      data-tour={`slot-${slotKey}`}
      onClick={() => onSelect(slotKey)}
      aria-label={`${name}: ${base ? (runeword?.name ?? base.name) : locked ? 'locked' : 'empty'}`}
      className={`relative flex items-center justify-center overflow-hidden rounded-[3px] border transition-colors ${sizeClass} ${border} ${
        active
          ? 'border-accent-hot bg-accent-hot/10 ring-1 ring-accent-hot/40'
          : `${bg} hover:border-accent-deep`
      }`}
    >
      {base ? (
        sprite ? (
          <img
            src={sprite}
            alt=""
            draggable={false}
            className="pointer-events-none h-full w-full object-contain p-1 select-none"
            style={{ imageRendering: 'pixelated' }}
          />
        ) : (
          <span className={`text-xl leading-none ${RARITY_TEXT[base.rarity]}`}>◆</span>
        )
      ) : (
        <span className="px-0.5 text-center font-mono text-[7px] font-semibold uppercase tracking-[0.08em] text-faint/60 break-words">
          {locked ? '2H' : (emptyLabel ?? name)}
        </span>
      )}
      {stars > 0 && (
        <span
          className="absolute top-0 left-1 font-mono text-[8px] leading-tight text-accent-hot"
          style={{ textShadow: '0 1px 2px rgba(0,0,0,0.9)' }}
        >
          ★{stars}
        </span>
      )}
      {equipped && equipped.socketCount > 0 && (
        <span
          className="absolute right-1 bottom-0 font-mono text-[8px] leading-tight text-faint"
          style={{ textShadow: '0 1px 2px rgba(0,0,0,0.9)' }}
        >
          {socketsFilled}/{equipped.socketCount}◇
        </span>
      )}
    </button>
  )

  if (!equipped) return button
  return (
    <ItemTooltip equipped={equipped} placement="right" className="block">
      {button}
    </ItemTooltip>
  )
}

export function EquipmentDoll({
  activeSlot,
  offhandLocked,
  onSelect,
}: {
  activeSlot: SlotKey | null
  offhandLocked: boolean
  onSelect: (s: SlotKey) => void
}) {
  const inventory = useBuild((s) => s.inventory)
  const disabledPotions = useBuild((s) => s.disabledPotions)
  const setPotionDisabled = useBuild((s) => s.setPotionDisabled)

  const dollSlots = gameConfig.slots.filter((s) => !s.key.startsWith('charm_'))
  const equippedCount = dollSlots.filter((s) => inventory[s.key]).length

  const cell = (slotKey: SlotKey, sizeClass: string, emptyLabel?: string) => (
    <SlotCell
      slotKey={slotKey}
      equipped={inventory[slotKey]}
      active={activeSlot === slotKey}
      locked={slotKey === 'offhand' && offhandLocked}
      onSelect={onSelect}
      sizeClass={sizeClass}
      emptyLabel={emptyLabel}
    />
  )

  return (
    <GearPanel
      dataTour="gear-doll"
      title="Equipment"
      trailing={
        <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
          <span className={equippedCount > 0 ? 'text-accent-hot' : 'text-muted'}>
            {equippedCount}
          </span>
          <span className="text-faint"> / {dollSlots.length}</span> equipped
        </span>
      }
    >
      <div
        className="mx-auto grid w-fit gap-3 rounded-[3px] border border-border-2 p-4"
        style={{
          gridTemplateColumns: '44px 96px 220px 96px',
          gridTemplateAreas: `
            "relics .      helmet  amulet"
            "relics weapon armor   offhand"
            "relics .      rings   ."
            "relics gloves potions boots"
          `,
          background: 'linear-gradient(180deg, #0c0804, #070302)',
          boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.7)',
        }}
      >
        <div style={{ gridArea: 'relics' }} className="flex flex-col items-center gap-2">
          {RELIC_SLOTS.map((k, i) => (
            <div key={k}>{cell(k, 'h-[44px] w-[44px]', `R${i + 1}`)}</div>
          ))}
        </div>

        <div style={{ gridArea: 'helmet' }} className="flex items-end justify-center">
          {cell('helmet', 'h-[88px] w-[88px]')}
        </div>
        <div style={{ gridArea: 'amulet' }} className="flex items-end justify-center">
          {cell('amulet', 'h-[44px] w-[44px]', 'AMU')}
        </div>

        <div style={{ gridArea: 'weapon' }} className="flex items-start justify-center">
          {cell('weapon', 'h-[176px] w-[96px]')}
        </div>
        <div style={{ gridArea: 'armor' }} className="flex items-center justify-center">
          {cell('armor', 'h-[150px] w-[120px]')}
        </div>
        <div style={{ gridArea: 'offhand' }} className="flex items-start justify-center">
          {cell('offhand', 'h-[176px] w-[96px]')}
        </div>

        <div style={{ gridArea: 'rings' }} className="flex items-center justify-center gap-2">
          {cell('ring_1', 'h-[44px] w-[44px]', 'RING')}
          {cell('belt', 'h-[44px] w-[68px]')}
          {cell('ring_2', 'h-[44px] w-[44px]', 'RING')}
        </div>

        <div style={{ gridArea: 'gloves' }} className="flex items-start justify-center">
          {cell('gloves', 'h-[88px] w-[88px]')}
        </div>
        <div style={{ gridArea: 'potions' }} className="flex items-start justify-center gap-2">
          {POTION_SLOTS.map((k, i) => {
            const equipped = inventory[k]
            const enabled = !disabledPotions[k]
            return (
              <div key={k} className="flex flex-col items-center gap-1">
                {cell(k, 'h-[96px] w-[30px]', `P${i + 1}`)}
                {equipped && (
                  <button
                    type="button"
                    onClick={() => setPotionDisabled(k, enabled)}
                    aria-label={`${slotName(k)} effects ${enabled ? 'on' : 'off'}`}
                    title={enabled ? 'Effects applied — click to disable' : 'Effects off — click to enable'}
                    className={`h-[10px] w-[10px] rounded-full border transition-colors ${
                      enabled
                        ? 'border-accent-hot bg-accent-hot'
                        : 'border-border-2 bg-transparent hover:border-accent-deep'
                    }`}
                    style={enabled ? { boxShadow: '0 0 6px rgba(224,184,100,0.5)' } : undefined}
                  />
                )}
              </div>
            )
          })}
        </div>
        <div style={{ gridArea: 'boots' }} className="flex items-start justify-center">
          {cell('boots', 'h-[88px] w-[88px]')}
        </div>
      </div>
    </GearPanel>
  )
}
