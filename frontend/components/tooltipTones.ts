export type TooltipTone =
  | 'common'
  | 'uncommon'
  | 'rare'
  | 'mythic'
  | 'satanic'
  | 'heroic'
  | 'angelic'
  | 'satanic_set'
  | 'unholy'
  | 'relic'
  | 'neutral'

export const TONE_BORDER: Record<TooltipTone, string> = {
  common: 'border-white/40',
  uncommon: 'border-sky-400/60',
  rare: 'border-accent/60',
  mythic: 'border-purple-400/60',
  satanic: 'border-red-500/60',
  heroic: 'border-lime-400/60',
  angelic: 'border-yellow-200/60',
  satanic_set: 'border-green-400/60',
  unholy: 'border-pink-400/60',
  relic: 'border-orange-300/60',
  neutral: 'border-border-2',
}

export const TONE_GLOW: Record<TooltipTone, string> = {
  common: 'shadow-[0_0_24px_-6px_rgba(255,255,255,0.18)]',
  uncommon: 'shadow-[0_0_24px_-6px_rgba(56,189,248,0.30)]',
  rare: 'shadow-[0_0_24px_-6px_rgba(201,165,90,0.32)]',
  mythic: 'shadow-[0_0_24px_-6px_rgba(192,132,252,0.30)]',
  satanic: 'shadow-[0_0_24px_-6px_rgba(239,68,68,0.30)]',
  heroic: 'shadow-[0_0_24px_-6px_rgba(163,230,53,0.30)]',
  angelic: 'shadow-[0_0_24px_-6px_rgba(254,240,138,0.30)]',
  satanic_set: 'shadow-[0_0_24px_-6px_rgba(74,222,128,0.30)]',
  unholy: 'shadow-[0_0_24px_-6px_rgba(244,114,182,0.30)]',
  relic: 'shadow-[0_0_24px_-6px_rgba(253,186,116,0.30)]',
  neutral: 'shadow-[0_0_24px_-6px_rgba(0,0,0,0.6)]',
}

export const TONE_TEXT: Record<TooltipTone, string> = {
  common: 'text-white',
  uncommon: 'text-sky-400',
  rare: 'text-accent-hot',
  mythic: 'text-purple-400',
  satanic: 'text-red-500',
  heroic: 'text-lime-400',
  angelic: 'text-yellow-200',
  satanic_set: 'text-green-400',
  unholy: 'text-pink-400',
  relic: 'text-orange-300',
  neutral: 'text-text',
}

export const TONE_RGB: Record<TooltipTone, string> = {
  common: '255,255,255',
  uncommon: '56,189,248',
  rare: '201,165,90',
  mythic: '192,132,252',
  satanic: '239,68,68',
  heroic: '163,230,53',
  angelic: '254,240,138',
  satanic_set: '74,222,128',
  unholy: '244,114,182',
  relic: '253,186,116',
  neutral: '160,160,160',
}
