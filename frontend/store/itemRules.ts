import { getItem } from '@data'

export const MAX_STARS = 5
export const HARD_SOCKET_CAP = 6
export const BONUS_SOCKET_MOD_ID = 'crystal_add_socket'
export const RAINBOW_MULTIPLIER = 1.5

export function hasBonusSocketMod(
  forgedMods?: { affixId: string }[] | null,
): boolean {
  return !!forgedMods?.some((m) => m.affixId === BONUS_SOCKET_MOD_ID)
}

export function maxSocketsFor(
  baseId: string,
  forgedMods?: { affixId: string }[] | null,
): number {
  const base = getItem(baseId)
  if (!base) return 0
  let cap = base.maxSockets ?? base.sockets ?? 0
  if (hasBonusSocketMod(forgedMods)) cap += 1
  return Math.min(cap, HARD_SOCKET_CAP)
}
