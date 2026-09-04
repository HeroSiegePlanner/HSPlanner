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


