export const WARMUP_WEIGHT = 0.15
export const SPRITES_WEIGHT = 0.85

export interface PhaseCount {
  done: number
  total: number
}

export interface BootProgress {
  pct: number
  status: string
}

const fraction = ({ done, total }: PhaseCount): number =>
  total > 0 ? Math.min(done, total) / total : 0

const label = (text: string, { done, total }: PhaseCount): string =>
  total > 0 ? `${text} · ${done}/${total}` : text

export function bootProgress(warmup: PhaseCount, sprites: PhaseCount): BootProgress {
  const pct = (WARMUP_WEIGHT * fraction(warmup) + SPRITES_WEIGHT * fraction(sprites)) * 100
  if (fraction(sprites) < 1) return { pct, status: label('Loading sprites', sprites) }
  if (fraction(warmup) < 1) return { pct, status: label('Loading game data', warmup) }
  return { pct, status: 'Ready' }
}
