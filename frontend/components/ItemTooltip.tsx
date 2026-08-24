import type { CSSProperties, ReactNode } from 'react'
import { useCalcResult } from '../hooks/useCalcResult'
import {
  detectRuneword,
  effectiveStars,
  getAffix,
  getItem,
  getItemImage,
} from '@data'
import { useBuild } from '../store/build'
import type { EquippedItem, ItemBase, RangedValue } from '../types'
import { rangedMax, rangedMin, shouldScaleImplicit } from '../utils/item/stats'
import { displayValuesNative } from '../utils/calc/bridge'
import type { AffixValueOutput } from '../utils/calc/bridge'
import Tooltip, {
  TooltipFooter,
  TooltipHeader,
  TooltipSection,
  TooltipSectionHeader,
} from './ui/Tooltip'
import { TONE_BORDER, TONE_GLOW, TONE_RGB, TONE_TEXT } from './tooltipTones'
import type { TooltipTone } from './tooltipTones'
import { augmentIconForId, socketableIconForName } from '../views/gear/lib/gearIcons'
import { buildItemTooltipModel, EQUIPPED_MARK, RARITY_TONE } from './itemTooltipModel'
import type {
  ItemTooltipModel,
  TooltipLine,
  TooltipLineStyle,
  TooltipSectionModel,
} from './itemTooltipModel'

interface Props {
  equipped: EquippedItem
  children: ReactNode
  placement?: 'right' | 'left' | 'top' | 'bottom'
  className?: string
  zIndex?: number
}

export default function ItemTooltip({
  equipped,
  children,
  placement = 'right',
  className,
  zIndex,
}: Props) {
  const base = getItem(equipped.baseId)
  if (!base) return <>{children}</>

  const runeword = detectRuneword(base, equipped.socketed)
  const tone: TooltipTone = runeword ? 'rare' : RARITY_TONE[base.rarity]

  return (
    <Tooltip
      tone={tone}
      placement={placement}
      className={className}
      zIndex={zIndex}
      follow
      content={<ItemTooltipBody equipped={equipped} base={base} />}
    >
      {children}
    </Tooltip>
  )
}

interface TooltipDisplayValues {
  implicitScaled: Record<string, [number, number]>
  skillRankScaled: Record<string, [number, number]>
  affixRanges: (AffixValueOutput | null)[]
}

const EMPTY_DISPLAY: TooltipDisplayValues = {
  implicitScaled: {},
  skillRankScaled: {},
  affixRanges: [],
}

function useItemDisplayValues(
  base: ItemBase,
  equipped: EquippedItem | undefined,
  scaleImplicit: boolean,
): TooltipDisplayValues | null {
  return useCalcResult<TooltipDisplayValues | null>(
    () => {
      const stars = effectiveStars(base.slot, equipped?.stars)
      const toPair = (v: RangedValue): [number, number] => [
        rangedMin(v),
        rangedMax(v),
      ]
      const implicitEntries =
        scaleImplicit && base.implicit ? Object.entries(base.implicit) : []
      const skillEntries = base.skillBonuses
        ? Object.entries(base.skillBonuses)
        : []
      const equippedAffixes = equipped?.affixes ?? []
      const affixDefs = equippedAffixes.map((eq) => getAffix(eq.affixId))
      const affixReqs = equippedAffixes
        .map((eq, i) => ({ eq, def: affixDefs[i] }))
        .filter((x) => x.def)
        .map((x) => ({ affix: x.def, roll: x.eq.roll ?? 0, stars }))
      const scaled = [
        ...implicitEntries.map(([k, v]) => ({
          value: toPair(v),
          statKey: k,
          stars,
        })),
        ...skillEntries.map(([, v]) => ({
          value: toPair(v),
          statKey: 'item_granted_skill_rank',
          stars,
        })),
      ]
      if (affixReqs.length === 0 && scaled.length === 0) {
        return EMPTY_DISPLAY
      }
      return displayValuesNative({ affixes: affixReqs, scaled }).then((res) => {
        const implicitScaled: Record<string, [number, number]> = {}
        implicitEntries.forEach(([k], i) => {
          implicitScaled[k] = res.scaled[i]!
        })
        const skillRankScaled: Record<string, [number, number]> = {}
        skillEntries.forEach(([name], i) => {
          skillRankScaled[name] = res.scaled[implicitEntries.length + i]!
        })
        const affixRanges: (AffixValueOutput | null)[] = []
        let cursor = 0
        for (const def of affixDefs) {
          affixRanges.push(def ? (res.affixes[cursor++] ?? null) : null)
        }
        return { implicitScaled, skillRankScaled, affixRanges }
      })
    },
    [base, equipped, scaleImplicit],
    null,
  )
}

export function ItemTooltipBody({
  equipped,
  base,
}: {
  equipped?: EquippedItem
  base: ItemBase
}) {
  const inventory = useBuild((s) => s.inventory)
  const runeword = equipped ? detectRuneword(base, equipped.socketed) : undefined
  const display = useItemDisplayValues(
    base,
    equipped,
    shouldScaleImplicit(!!runeword),
  )
  if (!display) return null

  const model = buildItemTooltipModel(base, equipped, {
    display,
    inventory: toModelInventory(inventory),
  })
  return <ItemTooltipView model={model} />
}

function toModelInventory(
  inventory: Record<string, EquippedItem | undefined>,
): Record<string, EquippedItem | null> {
  const out: Record<string, EquippedItem | null> = {}
  for (const [slot, item] of Object.entries(inventory)) {
    out[slot] = item ?? null
  }
  return out
}

const LINE_STYLE_CLASS: Record<TooltipLineStyle, string> = {
  implicit: 'text-accent-hot',
  affix: 'text-yellow-300',
  'affix-missing': 'text-yellow-300/90 italic',
  unholy: 'text-pink-300',
  'unholy-missing': 'text-pink-300/90 italic',
  runeword: 'text-accent-hot',
  forged: 'text-stat-red',
  socket: 'text-accent',
  'set-active': 'text-green-300',
  'set-inactive': 'text-muted/70',
  'set-items': 'text-muted/70',
  proc: 'text-emerald-300',
  special: 'text-accent-hot',
  unsupported: `${TONE_TEXT.angelic} opacity-70`,
  muted: 'text-muted',
}

function badgeTitle(style: TooltipLineStyle): string {
  return style === 'implicit'
    ? 'Custom value (overrides base implicit)'
    : 'Custom value (overrides tier+roll)'
}

function ItemTooltipView({ model }: { model: ItemTooltipModel }) {
  return (
    <>
      <TooltipHeader
        tone={model.tone}
        title={model.name}
        subtitle={model.typeLine}
        image={getItemImage(model.imageId)}
      />
      {model.sections.map((section, i) => (
        <TooltipSection key={i}>
          <SectionContent section={section} />
        </TooltipSection>
      ))}
      {model.footer && <TooltipFooter>{model.footer}</TooltipFooter>}
    </>
  )
}

function SectionContent({ section }: { section: TooltipSectionModel }) {
  return (
    <>
      {section.header && (
        <TooltipSectionHeader
          tone={section.header.tone}
          trailing={section.header.trailing}
        >
          {section.header.text}
        </TooltipSectionHeader>
      )}
      <SectionLines lines={section.lines} />
      {section.footnote && (
        <p className="text-[10px] text-muted/70 italic mt-1">{section.footnote}</p>
      )}
    </>
  )
}

function SectionLines({ lines }: { lines: TooltipLine[] }) {
  const first = lines[0]
  if (!first) return null
  if (first.kind === 'row') return <BaseStatRows lines={lines} />
  if (first.kind === 'entry') {
    if (first.style === 'set-active' || first.style === 'set-inactive') {
      return <SetEntries lines={lines} />
    }
    if (first.style === 'proc') return <ProcEntries lines={lines} />
    if (first.style === 'socket') return <SocketEntries lines={lines} />
    return <GrantedSkillEntries lines={lines} />
  }
  if (first.style === 'muted') return <DescriptionLines lines={lines} />
  return <TextLines lines={lines} />
}

function BaseStatRows({ lines }: { lines: TooltipLine[] }) {
  return (
    <ul className="grid grid-cols-2 gap-x-4 gap-y-0.5 text-[12px] tabular-nums">
      {lines.map((line, i) =>
        line.kind === 'row' ? (
          <li key={i} className="contents">
            <span className="text-muted">{line.label}</span>
            <span className="text-text text-right font-medium">{line.value}</span>
          </li>
        ) : null,
      )}
    </ul>
  )
}

function TextLines({ lines }: { lines: TooltipLine[] }) {
  return (
    <ul className="space-y-0.5 text-[12px]">
      {lines.map((line, i) =>
        line.kind === 'text' ? (
          <li key={i} className={LINE_STYLE_CLASS[line.style]}>
            {line.text}
            {line.badge && (
              <span
                className="ml-1 font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot/70"
                title={badgeTitle(line.style)}
              >
                {line.badge}
              </span>
            )}
          </li>
        ) : null,
      )}
    </ul>
  )
}

function DescriptionLines({ lines }: { lines: TooltipLine[] }) {
  return (
    <>
      {lines.map((line, i) =>
        line.kind === 'text' ? (
          <p
            key={i}
            className={
              i === 0
                ? 'text-[11px] text-muted italic'
                : 'text-[11px] text-muted italic mt-1'
            }
          >
            {line.text}
          </p>
        ) : null,
      )}
    </>
  )
}

function GrantedSkillEntries({ lines }: { lines: TooltipLine[] }) {
  return (
    <ul className="space-y-1 text-[11px]">
      {lines.map((line, i) =>
        line.kind === 'entry' ? (
          <li key={i}>
            <div className="flex items-center gap-1.5 text-accent-hot text-[12px]">
              {line.icon && augmentIconForId(line.icon) && (
                <img
                  src={augmentIconForId(line.icon)}
                  alt=""
                  className="h-[18px] w-[18px] shrink-0 object-contain"
                  style={{ imageRendering: 'pixelated' }}
                />
              )}
              {line.title}
              {line.suffix && (
                <>
                  {' '}
                  <span className="text-muted text-[10px]">{line.suffix}</span>
                </>
              )}
            </div>
            {line.desc && (
              <div className="text-muted text-[10px] italic leading-snug">
                {line.desc}
              </div>
            )}
            {line.lines.length > 0 && (
              <ul className="mt-0.5 ml-2 space-y-0.5 text-text/80">
                {line.lines.map((l, j) => (
                  <li key={j} className="text-[11px]">
                    {l}
                  </li>
                ))}
              </ul>
            )}
          </li>
        ) : null,
      )}
    </ul>
  )
}

function SocketEntries({ lines }: { lines: TooltipLine[] }) {
  return (
    <ul className="space-y-1.5 text-[11px]">
      {lines.map((line, i) => {
        if (line.kind !== 'entry') return null
        const iconUrl = line.icon ? socketableIconForName(line.icon) : undefined
        return (
          <li key={i} className="flex items-start gap-1.5">
            {iconUrl && (
              <img
                src={iconUrl}
                alt=""
                className="mt-px h-[18px] w-[18px] shrink-0 object-contain"
              />
            )}
            <div className="min-w-0">
              <div className="text-[11px] text-muted">{line.title}</div>
              <ul className="space-y-0.5 text-accent-hot">
                {line.lines.map((l, j) => (
                  <li key={j}>{l}</li>
                ))}
              </ul>
            </div>
          </li>
        )
      })}
    </ul>
  )
}

function SetEntries({ lines }: { lines: TooltipLine[] }) {
  return (
    <ul className="space-y-1">
      {lines.map((line, i) =>
        line.kind === 'entry' ? (
          <li
            key={i}
            className={`text-[11px] ${LINE_STYLE_CLASS[line.style ?? 'set-inactive']} ${
              line.style === 'set-items' ? 'mt-2 border-t border-white/10 pt-2' : ''
            }`}
          >
            <div className="text-[10px] uppercase tracking-[0.12em]">{line.title}</div>
            <ul className="ml-1 space-y-0.5">
              {line.lines.map((d, j) => (
                <li key={j} className={d.startsWith(EQUIPPED_MARK) ? 'text-green-300' : undefined}>
                  {d}
                </li>
              ))}
            </ul>
          </li>
        ) : null,
      )}
    </ul>
  )
}

function ProcEntries({ lines }: { lines: TooltipLine[] }) {
  return (
    <ul className="space-y-1.5 text-[12px]">
      {lines.map((line, i) =>
        line.kind === 'entry' ? (
          <li key={i} className="text-emerald-300">
            {line.title}
            {line.desc && (
              <div className="text-[10px] text-muted italic leading-snug mt-0.5">
                {line.desc}
              </div>
            )}
          </li>
        ) : null,
      )}
    </ul>
  )
}

export type CompareState = 'equipped' | 'selected'

export function ItemCard({
  equipped,
  base,
  arcLabel,
  state,
  className,
}: {
  equipped?: EquippedItem
  base: ItemBase | undefined
  arcLabel?: ReactNode
  state?: CompareState
  className?: string
}) {
  const stateClass = state ? `compare-card is-${state}` : ''

  if (!base) {
    return (
      <div
        className={`relative bg-panel border border-dashed border-border rounded-sm ${stateClass} ${className ?? ''}`}
      >
        {state && arcLabel && (
          <div className="compare-arc" style={arcStyle('neutral')}>
            {arcLabel}
          </div>
        )}
        <div className="flex items-center justify-center text-center px-3 py-6">
          <p className="text-[11px] text-faint italic">empty slot</p>
        </div>
      </div>
    )
  }
  const runeword = equipped ? detectRuneword(base, equipped.socketed) : undefined
  const tone: TooltipTone = runeword ? 'rare' : RARITY_TONE[base.rarity]
  const overflow = state ? '' : 'overflow-hidden'

  return (
    <div
      className={`relative bg-panel border ${TONE_BORDER[tone]} ${TONE_GLOW[tone]} rounded-sm ${overflow} ${stateClass} ${className ?? ''}`}
    >
      {state && arcLabel && (
        <div className="compare-arc" style={arcStyle(tone)}>
          {arcLabel}
        </div>
      )}
      <ItemTooltipBody equipped={equipped} base={base} />
    </div>
  )
}

function arcStyle(tone: TooltipTone): CSSProperties {
  const rgb = TONE_RGB[tone]
  return {
    '--arc-text': `rgb(${rgb})`,
    '--arc-border': `rgba(${rgb}, 0.6)`,
    '--arc-bg': `color-mix(in srgb, rgb(${rgb}) 18%, #14151b)`,
  } as CSSProperties
}

export { RARITY_TONE }
