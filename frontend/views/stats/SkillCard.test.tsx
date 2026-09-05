import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { SkillCard } from './SkillCard'
import type { Skill } from '../../types'
import { skills } from '@data'
import { computeAttackSkillDamageNative, computeSkillDamageNative } from '../../utils/nativeDamage'

vi.mock('../../utils/nativeDamage', () => ({
  computeSkillDamageNative: vi.fn().mockResolvedValue(null),
  computeAttackSkillDamageNative: vi.fn(async ({ skill }: { skill: Skill }) => skill.attackScaling ? {
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
  } : null),
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

describe('<SkillCard> Scorching Whip', () => {
  it.each([1, 20])('shows learned fire damage at rank %i', async (rank) => {
    const skill = skills.find((s) => s.id === 'scorching_whip')!
    vi.mocked(computeAttackSkillDamageNative).mockClear()
    vi.mocked(computeSkillDamageNative).mockClear().mockResolvedValue({
      effectiveRankMin: rank, effectiveRankMax: rank,
      baseMin: 15.2, baseMax: 15.2, flatMin: 0, flatMax: 0,
      synergyMinPct: 0, synergyMaxPct: 0,
      skillDamageMinPct: 0, skillDamageMaxPct: 0,
      extraDamagePct: 0, extraDamageSources: [],
      critChance: 0, critDamagePct: 0, critMultiplierAvg: 1,
      multicastChancePct: 0, multicastMultiplier: 1, projectileCount: 1,
      elementalBreakPct: 0, elementalBreakMultiplier: 1,
      enemyResistancePct: 0, resistanceIgnoredPct: 0,
      effectiveResistancePct: 0, resistanceMultiplier: 1,
      hitMin: 16, hitMax: 16, critMin: 16, critMax: 16,
      finalMin: 16, finalMax: 16, avgMin: 16, avgMax: 16,
    })
    render(
      <ul>
        <SkillCard
          skill={skill}
          attributes={{} as never}
          stats={{}}
          skillRanksByName={{}}
          skillsByNormalizedName={{}}
          itemSkillBonuses={{}}
          rankBonuses={{}}
          currentRank={rank}
          enemyConditions={{}}
          enemyResistances={{}}
          skillProjectiles={{}}
          subtreeScoped={{}}
          isMain
        />
      </ul>,
    )
    expect(await screen.findByText(/fire damage/i)).toBeInTheDocument()
    expect(screen.queryByText('Not learned')).not.toBeInTheDocument()
    expect(computeSkillDamageNative).toHaveBeenCalledWith(expect.objectContaining({ allocatedRank: rank }))
    expect(computeAttackSkillDamageNative).not.toHaveBeenCalled()
  })
})
