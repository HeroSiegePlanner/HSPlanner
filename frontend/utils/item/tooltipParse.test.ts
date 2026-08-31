import { describe, expect, test } from 'vitest'
import { parseTooltipLines } from './tooltipParse'

// Real ocrs output for engine/tests/fixtures/tooltips/tooltip1.png (Tundra
// Hunter's Long Coat) — mangled brackets and OCR noise included on purpose.
const TOOLTIP1 = `TUNDRA HUNTER'S LONG COAT
HEROIC BODY ARMOR
(GEM, GEM. GEM, GEM, GEM)
DEFENSE: 1333 [227] [210-2401
35% CHANCE WHEN STRUCK SET SAIL LEVEL 30
TEMPORARILY INCREASES YOUR COLD SKILL DAMAGE AND
MANA REPLENISH.
COLD SKILL DAMAGE 75%
MANA REPLENISH 65%
+487% ENHANCED DEFENSE 1450-525]
+3 TO ALL SKILLS [2-3]
+3 TO COLD SKILLS |3-51
AUGMENT: LETHAL TEMPO [LEVEL5|
INCREASES YOUR ATTACK SPEED AND CRITICAL STRIKE DAMAGE
FOR A SHORT PERIOD
ATTACK SPEED 80%
CRITICAL STRIKE DAMAGE 40%
EXTRA DAMAGE TO DEEP FROZEN MONSTERS 15% 115-25
+30 TO ALL ATTRIBUTES
+2560 TO ADDITIVE COLD DAMAGE
+100 TO COLD SKILL DAMAGE
COLD SKILL DAMAGE INCREASED BY 25% 125-40)
-23% TO ENEMY COLD RESISTANCE [15-30)
+40% TO COLD RESISTANCE 130-50]
SOCKETS (5) [3-61
IN THE STORMS OF A COLD AND FROZEN TUNDRA. A LONE HUNTER IS
STALKING HIS PREY.
B. Pick up TIER SS. REQUIRES LEVEL 94`.split('\n')

describe('parseTooltipLines — Tundra Hunter tooltip (real OCR)', () => {
  const result = parseTooltipLines(TOOLTIP1)

  test('matches the base item by fuzzy name', () => {
    expect(result.baseId).toBe('body_armor_heroic_tundra_hunter_s_long_coat')
    expect(result.equipped).not.toBeNull()
  })

  test('pins ranged implicit rolls from shown values', () => {
    const ov = result.equipped?.implicitOverrides ?? {}
    expect(ov.enhanced_defense).toBe(487)
    expect(ov.all_skills).toBe(3)
    expect(ov.cold_skills).toBe(3)
    expect(ov.extra_damage_frozen).toBe(15)
    expect(ov.cold_skill_damage).toBe(25)
    expect(ov.ignore_cold_res).toBe(23)
    expect(ov.cold_resistance).toBe(40)
  })

  test('fixed implicit equal to base needs no override', () => {
    expect(result.equipped?.implicitOverrides?.all_attributes).toBeUndefined()
  })

  test('reads socket count from Sockets line', () => {
    expect(result.equipped?.socketCount).toBe(5)
    expect(result.equipped?.socketed).toHaveLength(5)
  })

  test('reads augment name and level', () => {
    expect(result.equipped?.augment).toEqual({ id: 'lethal_tempo', level: 5 })
  })

  test('pins granted skill level from proc line', () => {
    expect(result.equipped?.skillBonusOverrides).toEqual({ 'Set Sail': 30 })
  })

  test('recognizes socketed gems from leftover fixed lines (5× Pristine Sapphire)', () => {
    expect(result.equipped?.socketed).toEqual(
      new Array(5).fill('gem_pristine_sapphire'),
    )
    const gemLines = result.lines.filter((l) =>
      /ADDITIVE COLD DAMAGE|\+100 TO COLD SKILL DAMAGE/i.test(l.text),
    )
    expect(gemLines).toHaveLength(2)
    for (const l of gemLines) {
      expect(l.status).toBe('matched')
      expect(l.detail).toMatch(/Pristine Sapphire/)
    }
    expect(result.lines.filter((l) => l.status === 'warning')).toHaveLength(0)
  })

  test('ignores flavor, proc description and HUD noise', () => {
    const ignored = result.lines.filter((l) => l.status === 'ignored')
    expect(ignored.some((l) => /STALKING HIS PREY/i.test(l.text))).toBe(true)
    expect(ignored.some((l) => /REQUIRES LEVEL/i.test(l.text))).toBe(true)
    expect(ignored.some((l) => /TEMPORARILY INCREASES/i.test(l.text))).toBe(true)
  })

  test('imports no random affixes on a heroic item', () => {
    expect(result.equipped?.affixes).toEqual([])
  })
})

// Real ocrs output for tooltip2.png (Grimbone's Visage).
const TOOLTIP2 = `GRIMBONE'S VISAGE
HEROIC HELMET
(GEM, GEM, GEM, GEM)
DEFENSF: 135 [63][60-80]
+113% ENHANCED DEFENSE 110-140])
+2 TO ALL SKILLS [2-3]
AILMENT DAMAGE INCREASED BY 17% [15-20]
+20 TO INTFLLIGFNCF [15-25]
+2048 TO ADDITIVE COLD DAMAGF
+80 TO COLD SKILL DAMAGE
MAGIC SKILL DAMAGE INCREASED BY 36% [20-50]
REPLENISH MANA 115% 100-150]
+600 TO MANA (BASED ON LEVEL) [4-8]
+50% TO ALL RESISTANCES
SOCKETS (4) 11-4)
GRIMBONE THE HIVEMIND OF TORMENTED SOULS,
FACE OF TERROR TO ALL THOSE WHO MEET THEIR FATE
IN NIFLHEL.
TIER SS. REQUIRES LEVEL 100
2E`.split('\n')

describe('parseTooltipLines — Grimbone tooltip (real OCR)', () => {
  const result = parseTooltipLines(TOOLTIP2)

  test('matches base despite OCR typos in stat lines', () => {
    expect(result.baseId).toBe('helmet_heroic_grimbone_s_visage')
  })

  test('pins ranged implicits, including E→F OCR confusion', () => {
    const ov = result.equipped?.implicitOverrides ?? {}
    expect(ov.enhanced_defense).toBe(113)
    expect(ov.all_skills).toBe(2)
    expect(ov.ailment_damage_all).toBe(17)
    expect(ov.to_intelligence).toBe(20)
    expect(ov.magic_skill_damage).toBe(36)
  })

  test('fixed implicit equal to base needs no override', () => {
    expect(result.equipped?.implicitOverrides?.all_resistances).toBeUndefined()
  })

  test('recognizes forged crystal mod by value inside crystal range', () => {
    expect(result.equipped?.forgedMods).toEqual([
      {
        affixId: 'crystal_satanic_mana_based_on_level',
        tier: 1,
        roll: 0.5,
      },
    ])
  })

  test('sockets read despite mangled brackets', () => {
    expect(result.equipped?.socketCount).toBe(4)
  })

  test('fills sockets with 4× Pristine Sapphire inferred from gem lines', () => {
    expect(result.equipped?.socketed).toEqual(
      new Array(4).fill('gem_pristine_sapphire'),
    )
  })

  test('pins Replenish Mana implicit despite missing left bracket in OCR', () => {
    expect(result.equipped?.implicitOverrides?.mana_replenish).toBe(115)
    expect(result.lines.filter((l) => l.status === 'warning')).toHaveLength(0)
  })
})

// Real ocrs output for tooltip3.png (Gryphon's Claw).
const TOOLTIP3 = `GRYPHON'S CLAW
HEROIC AMULET
+12 TO EXECUTF |12-18]
DEAL EXTRA ATTACK DAMAGE TO MONSTERS BELOW 30% LIFE.
ATTACK DAMAGE 72%
+18% INCREASED CRITICAL STRIKE CHANCE [18-25]
+46% NCREASED CRITICAL STRIKE DAMAGF [40-50]
+23% CHANCE TO OPEN WOUNDS [15-25]
+27 TO ALL ATTRIBUTES 120-30]
TIER SS, REQUIRES LEVEL 100`.split('\n')

describe('parseTooltipLines — Gryphon amulet (real OCR)', () => {
  const result = parseTooltipLines(TOOLTIP3)

  test('matches base', () => {
    expect(result.baseId).toBe('amulet_heroic_gryphon_s_claw')
  })

  test('pins granted skill rank from "+12 to Execute"', () => {
    expect(result.equipped?.skillBonusOverrides?.Execute).toBe(12)
  })

  test('pins ranged implicits', () => {
    const ov = result.equipped?.implicitOverrides ?? {}
    expect(ov.crit_chance).toBe(18)
    expect(ov.crit_damage).toBe(46)
    expect(ov.all_attributes).toBe(27)
  })
})

// Real ocrs output for tooltip4.png (Ghostly Potion).
const TOOLTIP4 = `GHOSTLY POTION
POTION
CURRENTLY HAS 1 CHARGFS OUT OF 1
FLASK COOLDOWN 45 SECONDS
EFFECT DURATION 15 SECONDS
+5 TO PARALLEL DIMENSION
INCREASES YOUR SKILL DAMAGE FOR A SHORT PERIOD OF TIME
MAGIC SKILL DAMAGE 57%
PO
TH
ENHANCES MAGIC SKILL DAMAGE.
TIER S, REQUIRES LEVEL 1
TO`.split('\n')

describe('parseTooltipLines — potion tooltip (real OCR)', () => {
  const result = parseTooltipLines(TOOLTIP4)

  test('matches base and pins granted skill', () => {
    expect(result.baseId).toBe('potion_satanic_ghostly_potion')
    expect(result.equipped?.skillBonusOverrides).toEqual({
      'Parallel Dimension': 5,
    })
  })

  test('ignores charges, cooldown and HUD fragments', () => {
    const ignored = result.lines.filter((l) => l.status === 'ignored')
    expect(ignored.some((l) => /CHARGFS/i.test(l.text))).toBe(true)
    expect(ignored.some((l) => /FLASK COOLDOWN/i.test(l.text))).toBe(true)
  })
})

// Real ocrs output for tooltip5.png (Torch of Shadow) — "(JÖTUNN)" arrives as "U?TUNN)".
const TOOLTIP5 = `TORCH OF SHADOWS
HEROIC CHARM (1X2) (UNIQUE EQUIPPED)
5% CHANCE WHEN STRIKING ISHADOWFLAMES LEVEL3
UNLEASH FLAMES OF SHADOW TRAVELING FORWARD DEALING
ARCANE DAMAGE
ARCANE DAMAGE 1573072
+3 TO ALL SKILLS U?TUNN) [1-3]
+23 TO ALL ATTRIBUTES 120-30]
+8 TO LIGHT RADIUS
+20% TO ALL RESISTANCES 110-20)
Napd
d
TORCH FORGED AND LIGHTED IN THE SHADOW REALMS THAT HAS
SOME ODD PROPERTIES, SPREADING DARKNESS IN THE MORTAL WORLD
WHERE THERE SHOULD BE LIGHT:
TIER SS, REQUIRES LEVEL 100
TO`.split('\n')

describe('parseTooltipLines — Torch of Shadow charm (real OCR)', () => {
  const result = parseTooltipLines(TOOLTIP5)

  test('fuzzy-matches base name (Shadows vs Shadow)', () => {
    expect(result.baseId).toBe('charm_heroic_torch_of_shadow')
  })

  test('recovers class-restricted all skills from mangled "(JÖTUNN)"', () => {
    expect(result.equipped?.allSkillsClassId).toBe('jotunn')
    expect(result.equipped?.implicitOverrides?.all_skills_class).toBe(3)
  })

  test('pins proc granted skill level from bracketless proc line', () => {
    expect(result.equipped?.skillBonusOverrides?.Shadowflames).toBe(3)
  })

  test('pins remaining implicits and leaves fixed light radius alone', () => {
    const ov = result.equipped?.implicitOverrides ?? {}
    expect(ov.all_attributes).toBe(23)
    expect(ov.all_resistances).toBe(20)
    expect(ov.light_radius).toBeUndefined()
  })
})

describe('parseTooltipLines — failure modes', () => {
  test('returns null result with error when no item name matches', () => {
    const result = parseTooltipLines(['GIBBERISH XYZZY', '+10 TO NOTHING'])
    expect(result.baseId).toBeNull()
    expect(result.equipped).toBeNull()
    expect(result.errors.length).toBeGreaterThan(0)
  })

  test('handles empty input', () => {
    const result = parseTooltipLines([])
    expect(result.baseId).toBeNull()
    expect(result.equipped).toBeNull()
  })
})
