import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { SkillCard } from './SkillCard'
import type { Skill } from '../../types'

vi.mock('../../utils/nativeDamage', () => ({
  computeSkillDamageNative: vi.fn().mockResolvedValue(null),
  computeAttackSkillDamageNative: vi.fn().mockResolvedValue({
    effectiveRankMin: 50,
    effectiveRankMax: 64,
    weaponDamagePctMin: 658,
    weaponDamagePctMax: 850,
    skillFlatPhysMin: 0,
    skillFlatPhysMax: 0,
    attackRatingPctMin: 260,
    attackRatingPctMax: 340,
    synergyMinPct: 0,
    synergyMaxPct: 0,
    projectileCount: 1,
    weaponDamageMin: 100,
    weaponDamageMax: 200,
    enhancedDamageMinPct: 0,
    enhancedDamageMaxPct: 0,
    additivePhysicalMin: 0,
    additivePhysicalMax: 0,
    attackDamageMinPct: 0,
    attackDamageMaxPct: 0,
    crushingBlowModifier: 1.5,
    armorBreakPct: 0,
    deadlyBlowChance: 0,
    critChance: 0,
    critDamagePct: 0,
    critMultiplierAvg: 1,
    extraDamageSources: [],
    physicalHitMin: 1500,
    physicalHitMax: 2500,
    physicalAvgMin: 1800,
    physicalAvgMax: 3000,
    poisonHitMin: 0,
    poisonHitMax: 0,
    poisonAvgMin: 0,
    poisonAvgMax: 0,
    combinedHitMin: 1500,
    combinedHitMax: 2500,
    combinedAvgMin: 1800,
    combinedAvgMax: 3000,
    attacksPerSecondMin: 1.35,
    attacksPerSecondMax: 1.75,
    dpsMin: 2430,
    dpsMax: 5250,
  }),
}))

const heavyBall = {
  id: 'heavy_ball',
  name: 'Heavy Ball',
  classId: 'butcher',
  kind: 'active',
  damageType: 'physical',
  tags: ['Attack', 'Active', 'Melee', 'Strike'],
  maxRank: 20,
  ranks: [{ rank: 1, manaCost: 4 }],
  attackKind: 'attack',
  attackScaling: {
    weaponDamagePct: { base: 100, perLevel: 22 },
  },
} as unknown as Skill

describe('<SkillCard> attack skills', () => {
  it('shows weapon-scaled damage for a learned attack skill', async () => {
    render(
      <ul>
        <SkillCard
          skill={heavyBall}
          mcrRange={0}
          attributes={{} as never}
          stats={{}}
          skillRanksByName={{}}
          skillsByNormalizedName={{}}
          itemSkillBonuses={{}}
          rankBonuses={{}}
          currentRank={20}
          enemyConditions={{}}
          enemyResistances={{}}
          skillProjectiles={{}}
          subtreeScoped={{}}
          isMain={false}
          weapon={{ name: 'Test Maul', damageMin: 110, damageMax: 125 }}
        />
      </ul>,
    )
    expect((await screen.findAllByText(/1,500|1500/)).length).toBeGreaterThan(0)
    expect(screen.getByText(/physical damage/i)).toBeInTheDocument()
    expect(screen.getByText('Weapon damage')).toBeInTheDocument()
  })
})
