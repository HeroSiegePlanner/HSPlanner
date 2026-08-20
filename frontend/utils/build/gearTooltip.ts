import { buildItemTooltipModel } from '../../components/itemTooltipModel'
import type {
  ItemTooltipModel,
  TooltipLine,
  TooltipLineStyle,
  TooltipModelDeps,
  TooltipSectionModel,
} from '../../components/itemTooltipModel'
import type { TooltipTone } from '../../components/tooltipTones'
import { getItemImage } from '@data'
import type { EquippedItem, ItemBase } from '../../types'
import { itemImagePath } from './assetPaths'
import type { GearTooltip, GearTooltipLine, GearTooltipSection, Rarity, Tone } from './sharePayload'

const STYLE_TONE: Record<TooltipLineStyle, Tone> = {
  implicit: 'gold',
  affix: 'yellow',
  'affix-missing': 'yellow',
  unholy: 'pink',
  'unholy-missing': 'pink',
  runeword: 'gold',
  forged: 'red',
  socket: 'gold',
  'set-active': 'green',
  'set-inactive': 'muted',
  'set-items': 'muted',
  proc: 'good',
  special: 'gold',
  unsupported: 'muted',
  muted: 'muted',
}

function isMissingStyle(style: TooltipLineStyle): boolean {
  return style === 'affix-missing' || style === 'unholy-missing'
}

function toRarity(tone: TooltipTone): Rarity {
  return tone === 'neutral' ? 'common' : tone
}

function serializeLine(line: TooltipLine): GearTooltipLine {
  switch (line.kind) {
    case 'row':
      return { kind: 'row', label: line.label, value: line.value }
    case 'entry':
      return {
        kind: 'entry',
        title: line.title,
        ...(line.suffix !== undefined ? { suffix: line.suffix } : {}),
        ...(line.desc !== undefined ? { desc: line.desc } : {}),
        ...(line.lines.length > 0 ? { lines: line.lines } : {}),
        ...(line.style ? { tone: STYLE_TONE[line.style] } : {}),
      }
    case 'text': {
      const italic = line.italic === true || isMissingStyle(line.style)
      return {
        kind: 'text',
        text: line.text,
        tone: STYLE_TONE[line.style],
        ...(italic ? { italic: true } : {}),
        ...(line.small ? { small: true } : {}),
        ...(line.badge !== undefined ? { badge: line.badge } : {}),
      }
    }
  }
}

function serializeSection(section: TooltipSectionModel): GearTooltipSection {
  const { header, footnote } = section
  return {
    ...(header
      ? {
          header: {
            text: header.text,
            tone: header.tone,
            ...(header.trailing !== undefined ? { trailing: header.trailing } : {}),
          },
        }
      : {}),
    lines: section.lines.map(serializeLine),
    ...(footnote !== undefined ? { footnote } : {}),
  }
}

export function serializeTooltipModel(model: ItemTooltipModel): GearTooltip {
  const image = getItemImage(model.imageId) ? itemImagePath(model.imageId) : undefined
  return {
    name: model.name,
    rarity: toRarity(model.tone),
    typeLine: model.typeLine,
    ...(image ? { image } : {}),
    sections: model.sections.map(serializeSection),
    ...(model.footer ? { footer: model.footer } : {}),
  }
}

export function buildGearTooltip(
  base: ItemBase,
  equipped: EquippedItem,
  deps: TooltipModelDeps,
): GearTooltip {
  return serializeTooltipModel(buildItemTooltipModel(base, equipped, deps))
}
