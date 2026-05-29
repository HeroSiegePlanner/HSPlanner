import type { ItemRarity } from '../../../types'

const SOCKETABLE_ICONS = import.meta.glob<string>(
  '../../../assets/socketable/*.png',
  { eager: true, query: '?url', import: 'default' },
)
const SOCKETABLE_ICON_BY_NAME: Record<string, string> = {}
for (const [p, url] of Object.entries(SOCKETABLE_ICONS)) {
  const file = p.split('/').pop() ?? ''
  const key = file.replace(/_spr\.png$/i, '').replace(/_/g, ' ')
  SOCKETABLE_ICON_BY_NAME[key.toLowerCase()] = url
}
export function socketableIconForName(name: string): string | undefined {
  // Maps a gem/rune display name (e.g. "Chipped Amethyst", "Tul") to the bundled pixel-art PNG that the socket PickerModal renders next to the row. Mirrors JewelSocketModal's lookup so both modals share the same iconography.
  return SOCKETABLE_ICON_BY_NAME[name.toLowerCase()]
}

const GEM_TINT: Record<string, string> = {
  amethyst: '#c97acc',
  diamond: 'var(--color-text)',
  emerald: 'var(--color-stat-green)',
  ruby: 'var(--color-stat-red)',
  sapphire: 'var(--color-stat-blue)',
  topaz: 'var(--color-accent-hot)',
  skull: '#7a6a5a',
}
export function gemColorForName(name: string): string {
  // Picks a fallback diamond-tint colour for a gem/jewel based on the last word of its name (sapphire → blue, ruby → red, etc.). Used by the socket PickerRow when no PNG icon is bundled, so the row still reads as a colour-coded gem.
  const last = name.split(' ').slice(-1)[0]?.toLowerCase() ?? ''
  return GEM_TINT[last] ?? 'var(--color-faint)'
}

export function gemTintForRarity(rarity: ItemRarity | undefined): string {
  // Returns the primary tint color used for the diamond icon next to each gear-row in the slot modal. Mirrors the rarity palette used elsewhere in the app.
  switch (rarity) {
    case 'satanic':
      return '#d96b5a'
    case 'satanic_set':
      return '#74c98a'
    case 'angelic':
      return '#e0d36a'
    case 'unholy':
      return '#cf6db0'
    case 'heroic':
      return '#96c95a'
    case 'mythic':
      return '#a070c8'
    case 'rare':
      return '#c9a560'
    case 'uncommon':
      return '#5a8fc9'
    case 'relic':
      return '#d18a4a'
    default:
      return '#7a7468'
  }
}
