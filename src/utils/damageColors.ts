import type { DamageType } from '../types'

export interface DamageColor {
  text: string
  border: string
  pill: string
  glow: string
  rgb: string
}

export const DAMAGE_COLORS: Record<DamageType, DamageColor> = {
  physical: {
    text: 'text-text',
    border: 'border-white/60',
    pill: 'text-text/90 border-white/25 bg-white/[0.04]',
    glow: 'shadow-[0_0_12px_rgba(255,255,255,0.3)]',
    rgb: '212,207,191',
  },
  fire: {
    text: 'text-red-400',
    border: 'border-red-400',
    pill: 'text-red-400 border-red-500/40 bg-red-500/[0.08]',
    glow: 'shadow-[0_0_12px_rgba(248,113,113,0.45)]',
    rgb: '248,113,113',
  },
  cold: {
    text: 'text-sky-400',
    border: 'border-sky-400',
    pill: 'text-sky-400 border-sky-500/40 bg-sky-500/[0.08]',
    glow: 'shadow-[0_0_12px_rgba(56,189,248,0.45)]',
    rgb: '56,189,248',
  },
  lightning: {
    text: 'text-yellow-400',
    border: 'border-yellow-400',
    pill: 'text-yellow-400 border-yellow-500/40 bg-yellow-500/[0.08]',
    glow: 'shadow-[0_0_12px_rgba(250,204,21,0.45)]',
    rgb: '250,204,21',
  },
  poison: {
    text: 'text-green-400',
    border: 'border-green-400',
    pill: 'text-green-400 border-green-500/40 bg-green-500/[0.08]',
    glow: 'shadow-[0_0_12px_rgba(74,222,128,0.45)]',
    rgb: '74,222,128',
  },
  arcane: {
    text: 'text-purple-400',
    border: 'border-purple-400',
    pill: 'text-purple-400 border-purple-500/40 bg-purple-500/[0.08]',
    glow: 'shadow-[0_0_12px_rgba(192,132,252,0.45)]',
    rgb: '192,132,252',
  },
  explosion: {
    text: 'text-orange-400',
    border: 'border-orange-400',
    pill: 'text-orange-400 border-orange-500/40 bg-orange-500/[0.08]',
    glow: 'shadow-[0_0_12px_rgba(251,146,60,0.45)]',
    rgb: '251,146,60',
  },
  magic: {
    text: 'text-pink-400',
    border: 'border-pink-400',
    pill: 'text-pink-400 border-pink-500/40 bg-pink-500/[0.08]',
    glow: 'shadow-[0_0_12px_rgba(244,114,182,0.45)]',
    rgb: '244,114,182',
  },
}

export function skillHeroBg(type: DamageType): string {
  return `linear-gradient(135deg, rgba(${DAMAGE_COLORS[type].rgb},0.07), transparent 60%)`
}
