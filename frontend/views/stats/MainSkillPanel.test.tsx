import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { MainSkillSection } from './MainSkillPanel'
import type { AttackSkillDamageBreakdown } from '../../utils/item/stats'
import type { Skill } from '../../types'

const heavyBall = {
  id: 'heavy_ball',
  name: 'Heavy Ball',
  classId: 'butcher',
  kind: 'active',
  damageType: 'physical',
  tags: ['Attack', 'Active', 'Melee', 'Strike'],
  maxRank: 20,
  attackKind: 'attack',
} as unknown as Skill

const attackBreakdown: AttackSkillDamageBreakdown = {
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
  physicalHitMin: 1000,
  physicalHitMax: 2000,
  physicalAvgMin: 1200,
  physicalAvgMax: 2400,
  poisonHitMin: 0,
  poisonHitMax: 0,
  poisonAvgMin: 0,
  poisonAvgMax: 0,
  combinedHitMin: 1000,
  combinedHitMax: 2000,
  combinedAvgMin: 1200,
  combinedAvgMax: 2400,
  attacksPerSecondMin: 1.35,
  attacksPerSecondMax: 1.75,
  dpsMin: 1620,
  dpsMax: 4200,
}

function renderSection(attackDamage: AttackSkillDamageBreakdown | null) {
  return render(
    <MainSkillSection
      mainSkill={heavyBall}
      mainSkillRank={20}
      attributes={{} as never}
      skillRanksByName={{}}
      skillsByNormalizedName={{}}
      rankBonuses={{}}
      skillBreakdown={null}
      cost={undefined}

      weaponDamage={null}
      attackDamage={attackDamage}
    />,
  )
}

describe('<MainSkillSection> attack skills', () => {
  it('renders the attack damage hero instead of the weapon fallback', () => {
    renderSection(attackBreakdown)
    expect(screen.getAllByText('Hit damage').length).toBeGreaterThan(0)
    expect(screen.getByText(/attacks\/sec/)).toBeInTheDocument()
    expect(screen.getByText('Weapon damage')).toBeInTheDocument()
    expect(screen.getByText('Crushing blow')).toBeInTheDocument()
    expect(screen.queryByText(/no damage/)).not.toBeInTheDocument()
    expect(screen.getAllByText(/Attack damage/).length).toBeGreaterThan(0)
  })

  it('keeps the empty state when no breakdown exists at all', () => {
    renderSection(null)
    expect(screen.getByText(/Pick a main skill/)).toBeInTheDocument()
  })
})
