// Engine output (engine/src/calc/skill_cost.rs): per-skill mana / cast rate / sustain.
export interface EntityRate {
  base: number
  min: number
  max: number
}

export interface SkillCost {
  effRankMin: number
  effRankMax: number
  baseManaMin: number | null
  baseManaMax: number | null
  mcrMax: number
  baseRate: number | null
  speedMax: number
  manaMin: number | null
  manaMax: number | null
  lifeMin: number | null
  lifeMax: number | null
  castRateMin: number | null
  castRateMax: number | null
  entityRate: EntityRate | null
  manaPerSecMin: number | null
  manaPerSecMax: number | null
  manaRegenMin: number
  manaRegenMax: number
  sustainable: boolean
  unsustainable: boolean
  netMin: number | null
  netMax: number | null
  uptimeMin: number | null
  uptimeMax: number | null
}
