export const WARMUP_WEIGHT = 0.15
export const SPRITES_WEIGHT = 0.85

export interface BootProgress {
  pct: number
  status: string
}

export function warmupBootProgress(current: number, total: number): BootProgress {
  const fraction = total > 0 ? current / total : 1
  const pct = WARMUP_WEIGHT * fraction * 100
  return { pct, status: 'Loading game data' }
}

export function spriteBootProgress(loaded: number, total: number): BootProgress {
  const fraction = total > 0 ? loaded / total : 1
  const pct = (WARMUP_WEIGHT + SPRITES_WEIGHT * fraction) * 100
  const status =
    total > 0 ? `Loading sprites · ${loaded}/${total}` : 'Loading sprites'
  return { pct, status }
}
