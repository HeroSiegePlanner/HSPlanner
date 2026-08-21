export const ENTITY_KINDS = ['sentry', 'summon', 'guardian'] as const

export type EntityKind = (typeof ENTITY_KINDS)[number]
export type EntityRates = Record<EntityKind, number>

// The game never exposes the base rates, so each kind gets its own Config knob.
export const DEFAULT_ENTITY_RATE = 1

export function defaultEntityRates(): EntityRates {
  return { sentry: 1, summon: 1, guardian: 1 }
}

export function entityKindOfTag(tag: string): EntityKind | undefined {
  const kind = tag.toLowerCase()
  return ENTITY_KINDS.find((k) => k === kind)
}

// Builds saved before the split carried one rate for all three kinds.
export function entityRatesFrom(
  rates: Partial<EntityRates> | undefined,
  legacy: number | undefined,
): EntityRates {
  if (rates) return { ...defaultEntityRates(), ...rates }
  if (legacy === undefined) return defaultEntityRates()
  return { sentry: legacy, summon: legacy, guardian: legacy }
}

export interface EntityRate {
  base: number
  min: number
  max: number
}

// The entity's own cadence: the flat Config rate multiplied by "increased
// <kind> attack speed". Player cast/attack speed drives how fast they are
// spawned, never how fast they swing. Mirrors the entity branch in calc/build.rs.
export function entityAttackRate(
  kind: EntityKind,
  rates: EntityRates,
  increasedPct: [number, number],
  fixed = 0,
): EntityRate {
  // A subskill can pin the entity to a literal rate (C.Y.C.L.O.P.S. lasers tick
  // 4/s); pinned means pinned, so knob and speed bonuses drop out.
  if (fixed > 0) return { base: fixed, min: fixed, max: fixed }
  const base = rates[kind] ?? DEFAULT_ENTITY_RATE
  return {
    base,
    min: base * (1 + increasedPct[0] / 100),
    max: base * (1 + increasedPct[1] / 100),
  }
}

export function entityAttackSpeedKey(kind: EntityKind): string {
  return `${kind}_attack_speed`
}

export function entityAttackRateFixedKey(kind: EntityKind): string {
  return `${kind}_attack_rate_fixed`
}
