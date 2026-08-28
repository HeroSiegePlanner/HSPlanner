import type { SkillHitModel } from '../../types'

// Mirrors `hits_per_cast` in engine/src/calc/build.rs: the damage object clears its
// hit list every tick, so it lands one hit plus one per full tick of its life.
// `null` when the extraction never resolved a lifetime — the engine counts one hit.
export function hitsPerCast(
  model: SkillHitModel | undefined,
  durationBonusPct = 0,
): number | null {
  if (!model || model.lifetime === undefined) return null
  if (model.tickFrequency <= 0 || model.lifetime <= 0) return null
  const lifetime = model.lifetime * (1 + durationBonusPct / 100)
  return Math.floor(lifetime / model.tickFrequency) + 1
}
