// Boot splash progress is weighted by real cost: the Rust warm-up is one fast
// opaque call, while sprite fetches are hundreds of images, so sprites own most
// of the bar and drive it granularly.
export const WARMUP_WEIGHT = 0.15
export const SPRITES_WEIGHT = 0.85

export interface BootProgress {
  pct: number
  status: string
}

// Maps the Rust warm-up progress onto the warm-up slice of the bar. The raw
// node count is meaningless to users, so the label stays generic while the bar
// moves granularly instead of jumping in one opaque step.
export function warmupBootProgress(current: number, total: number): BootProgress {
  const fraction = total > 0 ? current / total : 1
  const pct = WARMUP_WEIGHT * fraction * 100
  return { pct, status: 'Loading game data' }
}

// Maps sprite load counts onto the sprite slice of the bar and surfaces the
// real count so the user can see honest progress instead of a 50% jump.
export function spriteBootProgress(loaded: number, total: number): BootProgress {
  const fraction = total > 0 ? loaded / total : 1
  const pct = (WARMUP_WEIGHT + SPRITES_WEIGHT * fraction) * 100
  const status =
    total > 0 ? `Loading sprites · ${loaded}/${total}` : 'Loading sprites'
  return { pct, status }
}
