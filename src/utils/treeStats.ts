import treeNodeInfo from '../data/tree-nodes.json'

export interface TreeNodeInfo {
  t: string
  n: string
  l: string[]
  g?: string[]
  note?: string
}

export const TREE_NODE_INFO = treeNodeInfo as Record<string, TreeNodeInfo>

export const TREE_WARP_IDS = new Set<number>(
  Object.entries(TREE_NODE_INFO)
    .filter(([, info]) => info.n === 'warp')
    .map(([id]) => Number(id)),
)

export const TREE_JEWELRY_IDS = new Set<number>(
  Object.entries(TREE_NODE_INFO)
    .filter(([, info]) => info.n === 'jewelry')
    .map(([id]) => Number(id)),
)

export type SelfConditionKey = 'crit_chance_below_40' | 'life_below_40'

export interface ParsedMod {
  key: string
  value: number
  selfCondition?: SelfConditionKey
}

interface ParseRule {
  test: RegExp
  build: (m: RegExpMatchArray) => ParsedMod | null
}

interface SelfConditionRule {
  test: RegExp
  condition: SelfConditionKey
}

const SELF_CONDITION_SUFFIXES: SelfConditionRule[] = [
  {
    test: /\s+when\s+critical\s+strike\s+chance\s+is\s+below\s+40%$/i,
    condition: 'crit_chance_below_40',
  },
  {
    test: /\s+(?:when|while)\s+(?:current\s+life\s+is\s+)?below\s+40%(?:\s+of)?\s+maximum\s+life$/i,
    condition: 'life_below_40',
  },
]

const WEAPON_CONTEXT_SUFFIX =
  /\s+(?:when|while)\s+(?:using|wielding|dual\s+wielding)\s+(?:a\s+|an\s+)?(?:two\s+handed\s+)?(?:melee\s+)?(?:axe[s]?|sword[s]?|bow[s]?|gun[s]?|wand[s]?|staff(?:\s+or\s+a\s+cane)?|cane|shield[s]?|throwing\s+weapon[s]?|two\s+handed\s+weapon|two\s+handed\s+melee\s+weapon)$/i

function stripWeaponContext(line: string): string {
  return line.replace(WEAPON_CONTEXT_SUFFIX, '').trimEnd()
}

export const SELF_CONDITION_LABELS: Record<SelfConditionKey, string> = {
  crit_chance_below_40: 'Critical Strike Chance is below 40% (auto)',
  life_below_40: 'Current Life is below 40% of Maximum',
}

export const SELF_CONDITION_KEYS: SelfConditionKey[] = [
  'crit_chance_below_40',
  'life_below_40',
]

function stripSelfCondition(line: string): {
  base: string
  selfCondition: SelfConditionKey | undefined
} {
  for (const rule of SELF_CONDITION_SUFFIXES) {
    if (rule.test.test(line)) {
      return {
        base: line.replace(rule.test, '').trimEnd(),
        selfCondition: rule.condition,
      }
    }
  }
  return { base: line, selfCondition: undefined }
}

export const ELEMENTS = ['arcane', 'cold', 'fire', 'lightning', 'poison'] as const
type Element = (typeof ELEMENTS)[number]

const ELEMENT_RE = ELEMENTS.join('|')

function num(s: string): number {
  const cleaned = s.replace(/^\+/, '')
  return Number(cleaned)
}

const RULES: ParseRule[] = [
  {
    test: /^([+\-\d.]+)\s+to\s+Maximum\s+Life$/i,
    build: (m) => ({ key: 'life', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Maximum\s+Mana$/i,
    build: (m) => ({ key: 'mana', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Maximum\s+Life$/i,
    build: (m) => ({ key: 'increased_life', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Maximum\s+Mana$/i,
    build: (m) => ({
      key: m[2] ? 'increased_mana_more' : 'increased_mana',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Mana$/i,
    build: (m) => ({ key: 'increased_mana', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Strength$/i,
    build: (m) => ({ key: 'to_strength', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Strength$/i,
    build: (m) => ({ key: 'to_strength', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Dexterity$/i,
    build: (m) => ({ key: 'to_dexterity', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Dexterity$/i,
    build: (m) => ({ key: 'to_dexterity', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Intelligence$/i,
    build: (m) => ({ key: 'to_intelligence', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Energy$/i,
    build: (m) => ({ key: 'to_energy', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Vitality$/i,
    build: (m) => ({ key: 'to_vitality', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Armor$/i,
    build: (m) => ({ key: 'to_armor', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+All\s+Attributes$/i,
    build: (m) => ({ key: 'all_attributes', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+All\s+Attributes$/i,
    build: (m) => ({ key: 'increased_all_attributes', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?(Strength|Dexterity|Intelligence|Energy|Vitality|Armor)$/i,
    build: (m) => {
      const attr = m[3]!.toLowerCase()
      return {
        key: m[2] ? `increased_${attr}_more` : `increased_${attr}`,
        value: num(m[1]!),
      }
    },
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Defense$/i,
    build: (m) => ({ key: 'defense', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Movement\s+Speed$/i,
    build: (m) => ({
      key: m[2] ? 'movement_speed_more' : 'movement_speed',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Attack\s+Speed$/i,
    build: (m) => ({
      key: m[2] ? 'increased_attack_speed_more' : 'increased_attack_speed',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Faster\s+Cast\s+Rate$/i,
    build: (m) => ({
      key: m[2] ? 'faster_cast_rate_more' : 'faster_cast_rate',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+(?:to\s+)?Spell\s+Haste$/i,
    build: (m) => ({ key: 'skill_haste', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Critical\s+Strike\s+Damage$/i,
    build: (m) => ({
      key: m[2] ? 'crit_damage_more' : 'crit_damage',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Critical\s+Damage$/i,
    build: (m) => ({ key: 'crit_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+Critical\s+Strike\s+Chance$/i,
    build: (m) => ({ key: 'crit_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+Critically\s+Hit\s+with\s+Spells$/i,
    build: (m) => ({ key: 'spell_crit_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Spell\s+Critical\s+Damage$/i,
    build: (m) => ({ key: 'spell_crit_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+(?:to\s+)?All\s+Resistances$/i,
    build: (m) => ({ key: 'all_resistances', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+(Total\s+)?All\s+Resistances$/i,
    build: (m) => ({
      key: m[2] ? 'all_resistances_more' : 'all_resistances',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+(Total\s+)?All\s+Resistances$/i,
    build: (m) => ({
      key: m[2] ? 'all_resistances_more' : 'all_resistances',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+Maximum\s+All\s+Resistances$/i,
    build: (m) => ({ key: 'max_all_resistances', value: num(m[1]!) }),
  },
  {
    test: new RegExp(
      `^([+\\-\\d.]+)%\\s+to\\s+(${ELEMENT_RE})\\s+Resistance$`,
      'i',
    ),
    build: (m) => ({
      key: `${m[2]!.toLowerCase() as Element}_resistance`,
      value: num(m[1]!),
    }),
  },
  {
    test: new RegExp(
      `^([+\\-\\d.]+)%\\s+to\\s+Maximum\\s+(${ELEMENT_RE})\\s+Resistance$`,
      'i',
    ),
    build: (m) => ({
      key: `max_${m[2]!.toLowerCase() as Element}_resistance`,
      value: num(m[1]!),
    }),
  },
  {
    test: new RegExp(
      `^([+\\-\\d.]+)%\\s+to\\s+(${ELEMENT_RE})\\s+Absorb$`,
      'i',
    ),
    build: (m) => ({
      key: `${m[2]!.toLowerCase() as Element}_absorption`,
      value: num(m[1]!),
    }),
  },
  {
    test: new RegExp(
      `^([+\\-\\d.]+)%\\s+Increased\\s+(Total\\s+)?(${ELEMENT_RE})\\s+Skill\\s+Damage$`,
      'i',
    ),
    build: (m) => {
      const element = m[3]!.toLowerCase() as Element
      return {
        key: m[2]
          ? `${element}_skill_damage_more`
          : `${element}_skill_damage`,
        value: num(m[1]!),
      }
    },
  },
  {
    test: new RegExp(
      `^([+\\-\\d.]+)\\s+to\\s+(${ELEMENT_RE})\\s+Skill\\s+Damage$`,
      'i',
    ),
    build: (m) => ({
      key: `flat_${m[2]!.toLowerCase() as Element}_skill_damage`,
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Magic\s+Skill\s+Damage$/i,
    build: (m) => ({
      key: m[2] ? 'magic_skill_damage_more' : 'magic_skill_damage',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Magic\s+Skill\s+Damage$/i,
    build: (m) => ({ key: 'flat_skill_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Spell\s+Damage$/i,
    build: (m) => ({ key: 'spell_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Area\s+of\s+Effect(?:\s+(?:skill\s+)?radius(?:\s+of\s+all\s+skills)?)?$/i,
    build: (m) => ({
      key: m[2] ? 'area_of_effect_more' : 'area_of_effect',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Spell\s+Duration$/i,
    build: (m) => ({ key: 'spell_duration_pct', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Hit\s+Recovery$/i,
    build: (m) => ({ key: 'faster_hit_recovery', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Bleed(?:ing)?\s+Damage$/i,
    build: (m) => ({ key: 'increased_bleeding_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Bleed(?:ing)?\s+Frequency$/i,
    build: (m) => ({ key: 'increased_bleeding_frequency', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Poison(?:ed)?\s+Damage$/i,
    build: (m) => ({ key: 'increased_poisoned_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Burning\s+Damage$/i,
    build: (m) => ({ key: 'increased_burning_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Stasis\s+Damage$/i,
    build: (m) => ({ key: 'increased_stasis_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Ailment(?:\s+Tick)?\s+Frequency$/i,
    build: (m) => ({ key: 'increased_ailment_frequency', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+inflict\s+Bleeding\s+on\s+hit$/i,
    build: (m) => ({ key: 'chance_inflict_bleeding', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+inflict\s+Poison(?:ed)?\s+on\s+hit$/i,
    build: (m) => ({ key: 'chance_inflict_poisoned', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+inflict\s+Burning\s+on\s+hit$/i,
    build: (m) => ({ key: 'chance_inflict_burning', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+inflict\s+Stasis\s+on\s+hit$/i,
    build: (m) => ({ key: 'chance_inflict_stasis', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Maximum\s+Bleed(?:ing)?\s+Stacks$/i,
    build: (m) => ({ key: 'max_bleed_stacks', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Maximum\s+Poison(?:ed)?\s+Stacks$/i,
    build: (m) => ({ key: 'max_poisoned_stacks', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Maximum\s+Burning\s+Stacks$/i,
    build: (m) => ({ key: 'max_burning_stacks', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Maximum\s+Stasis\s+Stacks$/i,
    build: (m) => ({ key: 'max_stasis_stacks', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Maximum\s+Rage\s+Stacks$/i,
    build: (m) => ({ key: 'max_rage_stacks', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Maximum\s+Colossus\s+Stacks$/i,
    build: (m) => ({ key: 'max_colossus_stacks', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Life\s+Steal$/i,
    build: (m) => ({
      key: m[2] ? 'life_steal_more' : 'life_steal',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Mana\s+Steal$/i,
    build: (m) => ({ key: 'mana_steal', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Rate\s+of\s+Life\s+Steal$/i,
    build: (m) => ({ key: 'life_steal_rate', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increases\s+Rate\s+of\s+Mana\s+Steal$/i,
    build: (m) => ({ key: 'mana_steal_rate', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Damage\s+Mitigation$/i,
    build: (m) => ({ key: 'damage_mitigation', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Damage\s+Taken\s+Mitigated$/i,
    build: (m) => ({ key: 'damage_mitigation', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Incoming\s+Damage\s+is\s+mitigated$/i,
    build: (m) => ({ key: 'damage_mitigation', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+Physical\s+Damage\s+Reduction$/i,
    build: (m) => ({ key: 'physical_damage_reduction', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Damage\s+Taken\s+returned\s+to\s+the\s+Attacker$/i,
    build: (m) => ({ key: 'damage_return', value: num(m[1]!) }),
  },
  {
    test: /^All\s+Damage\s+Taken\s+Reduced\s+by\s+([+\-\d.]+)%$/i,
    build: (m) => ({ key: 'all_damage_taken_reduced_pct', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Damage\s+Returned$/i,
    build: (m) => ({ key: 'damage_return', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Life\s+Replenished\s+per\s+second$/i,
    build: (m) => ({ key: 'life_replenish', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Maximum\s+Life\s+Replenished\s+(?:Per|per)\s+[Ss]econd$/i,
    build: (m) => ({ key: 'life_replenish_pct', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+replenished\s+per\s+second$/i,
    build: (m) => ({ key: 'life_replenish_pct', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+(Total\s+)?(?:to\s+)?Mana\s+Replenish$/i,
    build: (m) => ({
      key: m[2] ? 'mana_replenish_more' : 'mana_replenish',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+All\s+Skills$/i,
    build: (m) => ({ key: 'all_skills', value: num(m[1]!) }),
  },
  {
    test: new RegExp(
      `^([+\\-\\d.]+)\\s+to\\s+(${ELEMENT_RE})\\s+Skills$`,
      'i',
    ),
    build: (m) => ({
      key: `${m[2]!.toLowerCase() as Element}_skills`,
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Physical\s+Skills$/i,
    build: (m) => ({ key: 'physical_skills', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Explosion\s+Skills$/i,
    build: (m) => ({ key: 'explosion_skills', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Summon\s+Skills$/i,
    build: (m) => ({ key: 'summon_skills', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Light\s+Radius$/i,
    build: (m) => ({ key: 'light_radius', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+Magic\s+Find$/i,
    build: (m) => ({ key: 'magic_find', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+(?:Increased\s+)?Experience\s+Gain$/i,
    build: (m) => ({ key: 'experience_gain', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Decreased\s+Crowd\s+Control\s+Diminish$/i,
    build: (m) => ({ key: 'cc_diminish_decrease', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Attack\s+Rating$/i,
    build: (m) => ({ key: 'attack_rating', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Attack\s+Rating$/i,
    build: (m) => ({ key: 'attack_rating_pct', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Physical\s+Damage$/i,
    build: (m) => ({ key: 'additive_physical_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+for\s+a\s+deadly\s+blow$/i,
    build: (m) => ({ key: 'deadly_blow', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+cast\s+an\s+additional\s+time\s+when\s+casting$/i,
    build: (m) => ({ key: 'multicast_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Damage\s+while\s+Unarmed$/i,
    build: (m) => ({ key: 'damage_unarmed', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Explosion\s+(?:area\s+of\s+effect\s+)?[Dd]amage$/i,
    build: (m) => ({ key: 'explosion_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Enhanced\s+Damage$/i,
    build: (m) => ({
      key: m[2] ? 'enhanced_damage_more' : 'enhanced_damage',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+(?:Melee\s+|Ranged\s+)?Enhanced\s+Damage$/i,
    build: (m) => ({ key: 'enhanced_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+Faster\s+Cast\s+Rate(?:\s+while\s+wielding\s+a\s+wand)?$/i,
    build: (m) => ({ key: 'faster_cast_rate', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Faster\s+Cast\s+Rate(?:\s+while\s+wielding\s+a\s+wand)?$/i,
    build: (m) => ({
      key: m[2] ? 'faster_cast_rate_more' : 'faster_cast_rate',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+Spell\s+Mana\s+Leech$/i,
    build: (m) => ({ key: 'mana_steal', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Elemental\s+Break\s+Inflicted\s+on\s+hit$/i,
    build: (m) => ({ key: 'elemental_break_on_strike', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Elemental\s+Break\s+Inflicted\s+on\s+spell\s+hit$/i,
    build: (m) => ({ key: 'elemental_break_on_spell', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+(?:to\s+)?Elemental\s+Break$/i,
    build: (m) => ({ key: 'elemental_break', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Armor\s+Break(?:\s+Inflicted\s+on\s+hit)?$/i,
    build: (m) => ({ key: 'armor_break_on_strike', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+(?:Maximum|Minimum)\s+Damage(?:\s+when\s+wielding\s+a\s+shield)?$/i,
    build: (m) => ({ key: 'attack_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Physical\s+Damage$/i,
    build: (m) => ({
      key: m[2] ? 'enhanced_damage_more' : 'enhanced_damage',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Defense$/i,
    build: (m) => ({
      key: m[2] ? 'enhanced_defense' : 'enhanced_defense',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Ranged\s+Projectile\s+Damage$/i,
    build: (m) => ({
      key: m[2] ? 'ranged_projectile_damage_more' : 'ranged_projectile_damage',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Projectile\s+Damage$/i,
    build: (m) => ({ key: 'ranged_projectile_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Area\s+of\s+Effect\s+Spell\s+Damage$/i,
    build: (m) => ({
      key: m[2] ? 'spell_aoe_damage_more' : 'spell_aoe_damage',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+(?:Increased\s+)?Spell\s+Area\s+of\s+Effect\s+Radius$/i,
    build: (m) => ({ key: 'spell_aoe_radius', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(?:Total\s+)?Area\s+of\s+Effect\s+Spell\s+Damage$/i,
    build: (m) => ({ key: 'spell_aoe_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Spell\s+Projectile\s+Damage$/i,
    build: (m) => ({
      key: m[2] ? 'two_handed_spell_projectile_damage' : 'spell_projectile_damage',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+(?:Increased\s+)?Spell\s+Projectile\s+Size$/i,
    build: (m) => ({ key: 'spell_projectile_size', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Additional\s+Spell\s+Projectile$/i,
    build: (m) => ({ key: 'additional_spell_projectile', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Extra\s+Spell\s+Projectiles$/i,
    build: (m) => ({ key: 'extra_spell_projectiles_pct', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Damaging\s+Aura\s+Effectiveness$/i,
    build: (m) => ({ key: 'damaging_aura_effectiveness', value: num(m[1]!) }),
  },
  {
    test: /^(?:([+\-\d.]+)%\s+)?Increased\s+Damaging\s+Aura\s+Radius$/i,
    build: (m) => ({
      key: 'damaging_aura_radius',
      value: m[1] ? num(m[1]) : 1,
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Damaging\s+)?Aura\s+Radius$/i,
    build: (m) => ({ key: 'damaging_aura_radius', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Aura\s+Skills$/i,
    build: (m) => ({ key: 'aura_skills', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Shield\s+Skills$/i,
    build: (m) => ({ key: 'shield_skills', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(?:Total\s+)?(?:Damage\s+by\s+)?Shield\s+Skill(?:\s+|s\s+)Damage$/i,
    build: (m) => ({ key: 'shield_skill_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Damage\s+by\s+Shield\s+Skills$/i,
    build: (m) => ({ key: 'shield_skill_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+radius\s+of\s+Shield\s+Skills$/i,
    build: (m) => ({ key: 'shield_skill_radius', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Sentry\s+Damage(?:\s+but\s+you\s+can\s+no\s+longer\s+deal\s+damage\s+your\s+self)?$/i,
    build: (m) => ({
      key: m[2] ? 'sentry_damage_more' : 'sentry_damage',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Sentry\s+Attack\s+Speed$/i,
    build: (m) => ({ key: 'sentry_attack_speed', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Sentry\s+Duration$/i,
    build: (m) => ({ key: 'sentry_duration', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+for\s+Sentries\s+to\s+fire\s+an\s+additional\s+projectile$/i,
    build: (m) => ({ key: 'sentry_extra_projectile_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Summon\s+Damage$/i,
    build: (m) => ({ key: 'summon_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Summon\s+Attack\s+Speed$/i,
    build: (m) => ({ key: 'summon_attack_speed', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Summon\s+Life$/i,
    build: (m) => ({ key: 'summon_life', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+Summon\s+Maximum\s+Life$/i,
    build: (m) => ({ key: 'summon_life', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Summon\s+Melee\s+Damage$/i,
    build: (m) => ({ key: 'summon_melee_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Summon\s+Projectile\s+Damage$/i,
    build: (m) => ({
      key: m[2] ? 'summon_projectile_damage_more' : 'summon_projectile_damage',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Summon\s+Projectile\s+Size$/i,
    build: (m) => ({ key: 'summon_projectile_size', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Summon\s+Attack\s+Radius$/i,
    build: (m) => ({ key: 'summon_attack_radius', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Summon\s+Splash\s+Damage(?:\s+around\s+the\s+target)?$/i,
    build: (m) => ({ key: 'summon_splash_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Maximum\s+Summon\s+Amount$/i,
    build: (m) => ({ key: 'summon_max_amount', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+for\s+summon\s+projectiles?\s+to\s+chain\s+on\s+hit$/i,
    build: (m) => ({ key: 'summon_chain_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+for\s+summon\s+projectile\s+to\s+fork\s+on\s+hit$/i,
    build: (m) => ({ key: 'summon_fork_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+on\s+summon\s+hit\s+to\s+knock\s+monsters\s+up\s+dealing\s+increased\s+damage$/i,
    build: (m) => ({ key: 'summon_knock_up_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Knock\s+Up\s+Damage$/i,
    build: (m) => ({ key: 'summon_knock_up_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Void\s+Blast\s+Damage$/i,
    build: (m) => ({ key: 'void_blast_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Guardian\s+Damage$/i,
    build: (m) => ({ key: 'guardian_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Guardian\s+Attack\s+Speed$/i,
    build: (m) => ({ key: 'guardian_attack_speed', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Guardian\s+Duration$/i,
    build: (m) => ({ key: 'guardian_duration', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+when\s+summoning\s+guardian\s+to\s+spawn\s+an\s+extra\s+guardian$/i,
    build: (m) => ({ key: 'extra_guardian_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Maximum\s+Summoned\s+Guardians$/i,
    build: (m) => ({ key: 'max_summoned_guardians', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Sand\s+Beam\s+Damage$/i,
    build: (m) => ({ key: 'sand_beam_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Orbiting\s+Skill\s+Damage$/i,
    build: (m) => ({ key: 'orbital_skill_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+(?:Increased\s+)?Total\s+Orbital\s+Spell\s+Damage$/i,
    build: (m) => ({ key: 'orbital_skill_damage_more', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Orbital\s+Spell\s+Damage$/i,
    build: (m) => ({ key: 'orbital_skill_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Orbiting\s+Skill\s+Speed$/i,
    build: (m) => ({ key: 'orbital_skill_speed', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Orbiting\s+Skill\s+Duration$/i,
    build: (m) => ({ key: 'orbital_skill_duration', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Orbital\s+Spell\s+Duration$/i,
    build: (m) => ({ key: 'orbital_skill_duration', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Orbiting\s+Skill\s+Size$/i,
    build: (m) => ({ key: 'orbital_skill_size', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Orbital\s+Spell\s+Size$/i,
    build: (m) => ({ key: 'orbital_skill_size', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+suppress\s+incoming\s+elemental\s+damage$/i,
    build: (m) => ({ key: 'elemental_suppression_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+elemental\s+suppression\s+effectiveness$/i,
    build: (m) => ({ key: 'elemental_suppression_effectiveness', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+Evade\s+incoming\s+damage$/i,
    build: (m) => ({ key: 'evade_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+Evade\s+projectiles$/i,
    build: (m) => ({ key: 'evade_projectile_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+evade\s+elemental\s+damage$/i,
    build: (m) => ({ key: 'evade_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+Dodge\s+Physical\s+Damage$/i,
    build: (m) => ({ key: 'dodge_physical_damage_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(Total\s+)?Ailment\s+Damage$/i,
    build: (m) => ({
      key: m[2] ? 'ailment_damage_all_more' : 'ailment_damage_all',
      value: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Skills\s+Damage\s+added\s+to\s+the\s+Ailments\s+damage$/i,
    build: (m) => ({ key: 'skill_damage_to_ailments', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Poison(?:ed)?\s+Frequency$/i,
    build: (m) => ({ key: 'increased_poisoned_frequency', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Knockback\s+Force$/i,
    build: (m) => ({ key: 'knockback_force', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Knockback\s+Damage$/i,
    build: (m) => ({ key: 'knockback_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Damage\s+when\s+knocking\s+monsters\s+into\s+terrain$/i,
    build: (m) => ({ key: 'knockback_terrain_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+when\s+attacking\s+to\s+fire\s+a\s+homing\s+missile$/i,
    build: (m) => ({ key: 'homing_missile_attack_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Homing\s+Missile\s+Damage$/i,
    build: (m) => ({ key: 'homing_missile_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+additional\s+Homing\s+Missile$/i,
    build: (m) => ({ key: 'homing_missile_count', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Homing\s+Missile\s+Explosion\s+area\s+of\s+effect\s+radius$/i,
    build: (m) => ({ key: 'homing_missile_explosion_radius', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+for\s+homing\s+missiles\s+to\s+unleash\s+a\s+shockwave\s+of\s+explosions$/i,
    build: (m) => ({ key: 'homing_missile_shockwave_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Socketable\s+Slot$/i,
    build: () => null,
  },
  {
    test: /^Path\s+to\s+any\s+Black\s+Hole$/i,
    build: () => null,
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Explosion\s+(?:area\s+of\s+effect\s+)?radius$/i,
    build: (m) => ({ key: 'explosion_aoe', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Damage\s+when\s+Dual\s+Wielding$/i,
    build: (m) => ({ key: 'damage_dual_wield', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Total\s+Damage\s+when\s+Dual\s+Wielding$/i,
    build: (m) => ({ key: 'damage_dual_wield_more', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Melee\s+Damage(?:\s+when\s+using\s+a\s+Shield)?$/i,
    build: (m) => ({ key: 'damage_with_shield', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Melee\s+Damage\s+to\s+Monsters\s+far\s+away\s+from\s+you$/i,
    build: (m) => ({ key: 'damage_far', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Melee\s+Damage\s+dealt\s+to\s+monsters\s+at\s+long\s+range\s+but\s+deal\s+less\s+damage\s+to\s+monsters\s+close\s+to\s+you$/i,
    build: (m) => ({ key: 'damage_far', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Ranged\s+Projectile\s+Damage\s+to\s+monsters\s+close\s+to\s+you\s+but\s+deal\s+less\s+damage\s+to\s+monsters\s+far\s+away$/i,
    build: (m) => ({ key: 'damage_close', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Damage\s+Reduction\s+when\s+wielding\s+a\s+Staff\s+or\s+a\s+Cane$/i,
    build: (m) => ({ key: 'two_handed_damage_reduction', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Damage\s+when\s+wielding\s+an\s+Axe$/i,
    build: (m) => ({ key: 'damage_with_axe', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Total\s+Attack\s+Rating\s+when\s+wielding\s+an\s+Axe$/i,
    build: (m) => ({ key: 'attack_rating_with_axe_pct', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Physical\s+Damage\s+when\s+dual\s+wielding\s+Axes$/i,
    build: (m) => ({ key: 'damage_to_terrain_flat_with_axes', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Attack\s+Radius\s+when\s+dual\s+wielding\s+Axes$/i,
    build: (m) => ({ key: 'attack_radius_dual_axes', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Attack\s+Radius$/i,
    build: (m) => ({ key: 'attack_radius', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(?:Melee\s+Attack\s+Damage|Melee\s+Damage)\s+when\s+using\s+a\s+Two\s+Handed\s+Melee\s+Weapon$/i,
    build: (m) => ({ key: 'damage_with_two_handed', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(?:Damage\s+when\s+using\s+a\s+Two\s+Handed\s+Weapon|Ailment\s+Damage\s+when\s+using\s+a\s+Two\s+Handed\s+Weapon)$/i,
    build: (m) => ({ key: 'damage_with_two_handed', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Spell\s+Projectile\s+Damage\s+when\s+wielding\s+a\s+Staff\s+or\s+a\s+Cane$/i,
    build: (m) => ({ key: 'two_handed_spell_projectile_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Magic\s+Skill\s+Damage\s+while\s+wielding\s+a\s+Wand$/i,
    build: (m) => ({ key: 'magic_skill_damage_with_wand', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+block\s+Physical\s+Damage\s+when\s+wielding\s+a\s+Staff\s+or\s+a\s+Cane$/i,
    build: (m) => ({ key: 'block_chance_physical_two_handed', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Melee\s+Attack\s+Range\s+when\s+using\s+a\s+Shield$/i,
    build: (m) => ({ key: 'melee_range', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Damage\s+Mitigation\s+when\s+using\s+a\s+Shield$/i,
    build: (m) => ({ key: 'damage_mitigation', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Total\s+Critical\s+Strike\s+Damage\s+when\s+using\s+a\s+Shield$/i,
    build: (m) => ({ key: 'crit_damage_more', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Vitality\s+when\s+wielding\s+a\s+shield$/i,
    build: (m) => ({ key: 'vitality_with_shield_flat', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Vitality\s+when\s+wielding\s+a\s+shield$/i,
    build: (m) => ({ key: 'vitality_with_shield', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Minimum\s+Damage\s+when\s+wielding\s+a\s+shield$/i,
    build: (m) => ({ key: 'min_damage_with_shield', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Maximum\s+Damage\s+when\s+wielding\s+a\s+shield$/i,
    build: (m) => ({ key: 'max_damage_with_shield', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Damage\s+Returned\s+when\s+wielding\s+a\s+Shield$/i,
    build: (m) => ({ key: 'damage_return', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Projectile\s+Damage\s+when\s+using\s+a\s+Gun$/i,
    build: (m) => ({ key: 'damage_with_gun', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Ranged\s+Projectile\s+Damage\s+when\s+using\s+a\s+Bow$/i,
    build: (m) => ({ key: 'damage_with_bow', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Projectile\s+Damage\s+when\s+using\s+a\s+Throwing\s+Weapon$/i,
    build: (m) => ({ key: 'damage_with_throwing', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+All\s+Resistances$/i,
    build: (m) => ({ key: 'all_resistances', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Sentry\s+Duration$/i,
    build: (m) => ({ key: 'sentry_duration', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Attack\s+Speed\s+at\s+Full\s+Life$/i,
    build: (m) => ({ key: 'attack_speed_full_life', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+(?:to\s+)?Increased\s+Total\s+Attack\s+Rating$/i,
    build: (m) => ({ key: 'attack_rating_pct', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Total\s+Damage\s+Return$/i,
    build: (m) => ({ key: 'damage_return_more', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Damage\s+Returned\s+against\s+Bosses$/i,
    build: (m) => ({ key: 'damage_returned_against_bosses', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+for\s+returned\s+damage\s+against\s+a\s+boss\s+to\s+echo\s+an\s+additional\s+time$/i,
    build: (m) => ({ key: 'returned_damage_echo_chance_boss', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+for\s+returned\s+damage\s+to\s+echo\s+an\s+additional\s+time(?:\s+till\s+failior)?$/i,
    build: (m) => ({ key: 'returned_damage_echo_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+return\s+the\s+damage\s+as\s+area\s+of\s+effect\s+damage$/i,
    build: (m) => ({ key: 'returned_damage_aoe_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+for\s+returned\s+damage\s+to\s+critically\s+hit$/i,
    build: (m) => ({ key: 'returned_damage_crit_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Damage\s+Returned\s+is\s+converted\s+into\s+Burning$/i,
    build: (m) => ({ key: 'returned_damage_to_burning', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Returned\s+Damage\s+is\s+dealt\s+as\s+Lightning\s+Damage$/i,
    build: (m) => ({ key: 'returned_damage_to_lightning', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+for\s+monsters\s+to\s+rest\s+in\s+peace$/i,
    build: (m) => ({ key: 'rest_in_peace_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+Maximum\s+Magic\s+Damage\s+Reduction$/i,
    build: (m) => ({ key: 'max_magic_damage_reduction', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+Maximum\s+Physical\s+Damage\s+Reduction$/i,
    build: (m) => ({ key: 'max_physical_damage_reduction', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Ranged\s+Projectile\s+Damage\s+per\s+stack\s+of\s+rage$/i,
    build: (m) => ({ key: 'ranged_damage_per_rage_stack', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Damage\s+Increased\s+per\s+Stack\s+of\s+Rage$/i,
    build: (m) => ({ key: 'damage_per_rage_stack', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(?:Ranged\s+)?Physical\s+Damage\s+per\s+500\s+mana$/i,
    build: (m) => {
      const isRanged = /Ranged/i.test(m[0]!)
      return {
        key: isRanged ? 'ranged_physical_per_500_mana' : 'additive_physical_per_500_mana',
        value: num(m[1]!),
      }
    },
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Magic\s+Skills?\s+Damage\s+per\s+750\s+points\s+in\s+Mana$/i,
    build: (m) => ({ key: 'magic_skill_damage_per_750_mana', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Charging\s+Damage\s+per\s+point\s+in\s+Strength$/i,
    build: (m) => ({ key: 'charging_damage_per_strength', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Maximum\s+Life\s+per\s+5\s+points\s+in\s+Strength$/i,
    build: (m) => ({ key: 'life_replenish_flat_strength', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Damage\s+Replenished\s+as\s+Life\s+when\s+struck$/i,
    build: (m) => ({ key: 'damage_to_life_replenish_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+replenished\s+when\s+evading$/i,
    build: (m) => ({ key: 'life_replenish_when_evading', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+replenished\s+when\s+struck$/i,
    build: (m) => ({ key: 'damage_to_life_replenish_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+replenished\s+when\s+suppressing$/i,
    build: (m) => ({ key: 'life_replenish_when_suppressing', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+replenish\s+\d+\s+life\s+and\s+\d+\s+mana\s+on\s+hit\s+with\s+ranged\s+attacks$/i,
    build: (m) => ({ key: 'life_replenish_mana_on_ranged_hit', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Total\s+Life\s+Replenish(?:\s+when\s+current\s+life\s+is\s+below\s+40%\s+of\s+Maximum\s+Life)?$/i,
    build: (m) => ({ key: 'life_replenish_more', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Life\s+Regeneration\s+from\s+Flasks$/i,
    build: (m) => ({ key: 'life_regen_flask', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Reduced\s+damage\s+taken\s+while\s+flask\s+regeneration$/i,
    build: (m) => ({ key: 'flask_damage_reduction', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+(?:Total\s+)?Life\s+Steal$/i,
    build: (m) => ({ key: /Total/i.test(m[0]!) ? 'life_steal_more' : 'life_steal', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Life\s+Stolen\s+Suppressed$/i,
    build: (m) => ({ key: 'life_steal_suppressed', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Maximum\s+Mana\s+regenerated\s+per\s+second$/i,
    build: (m) => ({ key: 'mana_regen_per_second', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Damage\s+Mitigation\s+for\s+a\s+short\s+duration\s+when\s+struck$/i,
    build: (m) => ({ key: 'damage_mitigation_when_struck', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+damage\s+dealt\s+by\s+Area\s+of\s+Effect\s+skills$/i,
    build: (m) => ({ key: 'area_of_effect', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+(?:Increased\s+Crushing\s+Blow\s+Chance|Chance\s+for\s+a\s+Crushing\s+Blow)$/i,
    build: (m) => ({ key: 'crushing_blow_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Crushing\s+Blow\s+Effectiveness$/i,
    build: (m) => ({ key: 'crushing_blow_effectiveness', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+effectiveness\s+of\s+Deadly\s+Blow$/i,
    build: (m) => ({ key: 'deadly_blow_effectiveness', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+deal\s+area\s+of\s+effect\s+damage\s+with\s+deadly\s+blow$/i,
    build: (m) => ({ key: 'deadly_blow_aoe_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Deadly\s+Blow\s+Area\s+of\s+Effect\s+Damage$/i,
    build: (m) => ({ key: 'deadly_blow_aoe_size', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+for\s+Projectiles\s+to\s+Fork(?:\s+on\s+Hit)?$/i,
    build: (m) => ({ key: 'fork_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Extra\s+Forking\s+Projectile$/i,
    build: (m) => ({ key: 'additional_forking_projectiles', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Extra\s+Projectile(?:\s+Fired\s+with\s+Ranged\s+Attacks)?$/i,
    build: (m) => ({ key: 'extra_ranged_projectiles', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Additional\s+projectile\s+fired\s+when\s+performing\s+a\s+ranged\s+attack$/i,
    build: (m) => ({ key: 'additional_projectile_fixed', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+fire\s+an\s+additional\s+projectile\s+when\s+performing\s+a\s+ranged\s+attack$/i,
    build: (m) => ({ key: 'extra_ranged_projectile_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+on\s+ranged\s+attack\s+to\s+perform\s+it\s+an\s+additional\s+time$/i,
    build: (m) => ({ key: 'ranged_extra_attack_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+unleash\s+an\s+additional\s+attack\s+or\s+projectile\s+on\s+attack$/i,
    build: (m) => ({ key: 'extra_attack_or_projectile_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Colossus\s+damage$/i,
    build: (m) => ({ key: 'colossus_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Life\s+Replenish\s+for\s+each\s+Colossus\s+stack$/i,
    build: (m) => ({ key: 'life_replenish_per_colossus_stack', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Death\s+Explosion\s+damage$/i,
    build: (m) => ({ key: 'death_explosion_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+for\s+monsters\s+slain\s+by\s+damage\s+return\s+to\s+explode\s+dealing\s+area\s+of\s+effect\s+damage$/i,
    build: (m) => ({ key: 'soul_ignition_explode_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Overflow\s+Effectiveness$/i,
    build: (m) => ({ key: 'overflow_effectiveness', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Agitation\s+Movement\s+Speed$/i,
    build: (m) => ({ key: 'agitation_movement_speed', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+gain\s+\d+%\s+Evasion\s+for\s+a\s+short\s+duration\s+when\s+struck$/i,
    build: (m) => ({ key: 'evasion_when_struck_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+gain\s+\d+%\s+Evasion\s+for\s+a\s+short\s+duration\s+after\s+being\s+hit$/i,
    build: (m) => ({ key: 'evasion_when_struck_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+(?:s\s+)?Evasion\s+Duration$/i,
    build: (m) => ({ key: 'evasion_duration', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+leave\s+a\s+cloud\s+of\s+poisonous\s+gas\s+when\s+struck$/i,
    build: (m) => ({ key: 'gas_cloud_when_struck_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Gas\s+Cloud\s+Poisoned\s+Damage$/i,
    build: (m) => ({ key: 'gas_cloud_poisoned_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+as\s+Explosion\s+Damage$/i,
    build: (m) => ({ key: 'summon_max_life_as_explosion', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Weapon\s+Throw\s+Damage$/i,
    build: (m) => ({ key: 'weapon_throw_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Temporal\s+Echo\s+Damage$/i,
    build: (m) => ({ key: 'temporal_echo_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Branch\s+Damage$/i,
    build: (m) => ({ key: 'branch_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Stampede\s+Damage\s+based\s+on\s+your\s+physical\s+damage$/i,
    build: (m) => ({ key: 'stampede_damage_per_physical', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+splash\s+damage\s+to\s+monsters\s+around\s+the\s+target$/i,
    build: (m) => ({ key: 'splash_damage_around_target', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+unleash\s+piercing\s+spikes\s+outwards\s+when\s+struck$/i,
    build: (m) => ({ key: 'damage_return_more', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Total\s+Damage\s+Dealt(?:\s+and\s+Damage\s+Taken)?$/i,
    build: (m) => ({ key: 'total_damage_dealt_and_taken', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Damage\s+and\s+Increased\s+Damage\s+Taken$/i,
    build: (m) => ({ key: 'damage_dealt_and_taken_amp', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Decreased\s+Monster\s+Crowd\s+Control\s+Immunity$/i,
    build: (m) => ({ key: 'cc_immunity_decrease_more', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Damage\s+dealt\s+to\s+monsters\s+with\s+Crowd\s+Control\s+Immunity$/i,
    build: (m) => ({ key: 'damage_to_cc_immune', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Monster\s+Damage\s+Over\s+Time\s+Immunity\s+Shattered$/i,
    build: (m) => ({ key: 'monster_dot_immunity_shattered', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Elemental\s+Weakness$/i,
    build: (m) => ({ key: 'elemental_weakness', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Target\s+Defense\s+Ignored$/i,
    build: (m) => ({ key: 'defense_ignored', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+(?:s\s+)?Stun\s+Duration$/i,
    build: (m) => ({ key: 'stun_duration', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Mana\s+Costs\s+are\s+taken\s+from\s+life\s+instead$/i,
    build: (m) => ({ key: 'mana_cost_paid_in_life', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Damage\s+Taken\s+is\s+drained\s+from\s+mana\s+instead$/i,
    build: (m) => ({ key: 'damage_drained_from_mana', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+deal\s+\d+%\s+area\s+of\s+effect\s+damage\s+on\s+bleed\s+tick$/i,
    build: (m) => ({ key: 'bleed_aoe_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Bleed\s+AoE\s+Damage$/i,
    build: (m) => ({ key: 'bleed_aoe_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s*Seconds?\s+to\s+Spell\s+Duration$/i,
    build: (m) => ({ key: 'spell_duration_seconds', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Spell\s+Area\s+of\s+Effect\s+Radius$/i,
    build: (m) => ({ key: 'spell_aoe_radius', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Spell\s+Projectile\s+Size$/i,
    build: (m) => ({ key: 'spell_projectile_size', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+for\s+Spell\s+Critical\s+hits\s+to\s+grant\s+Time\s+Surge\s+for\s+\d+\s+seconds$/i,
    build: (m) => ({ key: 'time_surge_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+on\s+cast\s+to\s+summon\s+a\s+Mirror\s+of\s+Odin\s+reaking\s+havoc\s+at\s+nearby\s+monsters$/i,
    build: (m) => ({ key: 'mirror_of_odin_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Lightning\s+Damage\s+scaling\s+from\s+all\s+elemental\s+sources$/i,
    build: (m) => ({ key: 'lightning_per_element', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+after\s+kill\s+to\s+recover\s+all\s+skill\s+cooldowns$/i,
    build: (m) => ({ key: 'cooldown_recovery_after_kill_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+(?:s\s+)?Cooldown\s+Recovered$/i,
    build: (m) => ({ key: 'cooldown_recovered_flat', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Extra\s+Damage\s+based\s+on\s+summons\s+maximum\s+life$/i,
    build: (m) => ({ key: 'extra_damage_from_summon_life', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Resistances\s+converted\s+to\s+Life$/i,
    build: () => null,
  },
  {
    test: /^([+\-\d.]+)%\s+Faster\s+Cast\s+Rate\s+per\s+Stack\s+of\s+Wizardry$/i,
    build: (m) => ({ key: 'wizardry_cast_rate', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+gain\s+a\s+stack\s+of\s+Mage\s+Guard\s+when\s+you\s+cast\s+a\s+spell\s+up\s+to\s+\d+\s+stacks$/i,
    build: (m) => ({ key: 'mage_guard_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Damage\s+Reduction\s+per\s+stack\s+of\s+Mage\s+Guard$/i,
    build: (m) => ({ key: 'mage_guard_damage_reduction', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Desert\s+Ripple\s+Damage$/i,
    build: (m) => ({ key: 'sand_ripple_damage', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+on\s+hit\s+for\s+Area\s+of\s+Effect\s+skills\s+to\s+pull\s+monsters\s+in$/i,
    build: (m) => ({ key: 'aoe_pull_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+cause\s+monsters\s+damaged\s+by\s+an\s+explosion\s+to\s+unleash\s+an\s+additional\s+explosion$/i,
    build: (m) => ({ key: 'additional_explosion_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+on\s+explosion\s+or\s+area\s+of\s+effect\s+skill\s+to\s+gain\s+a\s+stack\s+of\s+Ramping\s+Pulse\s+increasing\s+the\s+radius\s+of\s+area\s+of\s+effect\s+and\s+explosion\s+skills$/i,
    build: (m) => ({ key: 'ramping_pulse_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Maximum\s+Stacks$/i,
    build: (m) => ({ key: 'ramping_pulse_max_stacks', value: num(m[1]!) }),
  },
  {
    test: /^You\s+can\s+now\s+dual\s+wield\s+Two\s+Handed\s+Melee\s+Weapons$/i,
    build: () => ({ key: 'dual_wield_2h_melee', value: 1 }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Manacost$/i,
    build: (m) => ({ key: 'increased_manacost', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Ranged\s+Projectile\s+Damage\s+when\s+Critical\s+Strike\s+Chance\s+is\s+below\s+40%$/i,
    build: (m) => ({
      key: 'ranged_projectile_damage',
      value: num(m[1]!),
      selfCondition: 'crit_chance_below_40',
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Physical\s+Damage\s+while\s+below\s+40%\s+Maximum\s+Life$/i,
    build: (m) => ({
      key: 'enhanced_damage',
      value: num(m[1]!),
      selfCondition: 'life_below_40',
    }),
  },
  {
    test: /^You\s+can\s+no\s+longer\s+dodge\s+monster\s+attacks\s+but\s+also\s+cannot\s+be\s+stunned\s+or\s+frozen$/i,
    build: () => ({ key: 'force_field_protection', value: 1 }),
  },
  {
    test: /^Your\s+Maximum\s+All\s+Resistances\s+are\s+capped\s+to\s+([+\-\d.]+)%$/i,
    build: (m) => ({ key: 'max_all_resistances_cap', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+your\s+Light\s+Radius\s+added\s+as\s+Increased\s+All\s+Attributes$/i,
    build: (m) => ({ key: 'light_radius_to_attributes', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Total\s+Melee\s+Enhanced\s+Damage$/i,
    build: (m) => ({ key: 'enhanced_damage_melee_more', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Total\s+Ranged\s+Enhanced\s+Damage$/i,
    build: (m) => ({ key: 'enhanced_damage_ranged_more', value: num(m[1]!) }),
  },
  {
    test: /^Your\s+skill\s+weapon\s+type\s+restrictions\s+are\s+removed$/i,
    build: () => ({ key: 'skill_restrictions_removed', value: 1 }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+gain\s+an\s+orbiting\s+Bone\s+Fragment\s+when\s+struck$/i,
    build: (m) => ({ key: 'bone_fragment_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Return\s+Damage\s+inherited\s+by\s+Bone\s+Fragment$/i,
    build: (m) => ({ key: 'bone_fragment_inherit', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Life\s+Steal\s+is\s+now\s+instant\s+but\s+you\s+cannot\s+replenish\s+life\s+from\s+any\s+other\s+sources$/i,
    build: (m) => ({ key: 'vampirism_instant_life_steal', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+of\s+Incoming\s+Damage\s+is\s+ignored$/i,
    build: (m) => ({ key: 'damage_ignored_flat', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Damage\s+Taken\s+Reduced$/i,
    build: (m) => ({ key: 'damage_taken_reduced', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)\s+Extra\s+Jump\s+that\s+can\s+be\s+performed\s+mid\s+air$/i,
    build: (m) => ({ key: 'extra_jump_count', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Jump\s+Power$/i,
    build: (m) => ({ key: 'jumping_power', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+Damage\s+Mitigation\s+when\s+phasing\s+through\s+monsters$/i,
    build: (m) => ({ key: 'damage_mitigation', value: num(m[1]!) }),
  },
  {
    test: /^(?:\+0\s+)?Path\s+to\s+any\s+Black\s+Hole$/i,
    build: () => null,
  },
  {
    test: /^([+\-\d.]+)\s+to\s+Maximum\s+Combat\s+Mitigation\s+Stacks$/i,
    build: (m) => ({ key: 'max_combat_mitigation_stacks', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Damage\s+Taken\s+Recovered\s+as\s+Mana$/i,
    build: (m) => ({ key: 'damage_recouped_as_mana', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+to\s+Evade\s+projectiles\s+when\s+dual\s+wielding$/i,
    build: (m) => ({ key: 'evade_projectile_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Melee\s+Attack\s+Range$/i,
    build: (m) => ({ key: 'melee_range', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Melee\s+Attack\s+Range\s+when\s+using\s+a\s+Shield$/i,
    build: (m) => ({ key: 'melee_range', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Your\s+Mana\s+Costs\s+are\s+taken\s+from\s+life\s+instead$/i,
    build: (m) => ({ key: 'mana_cost_paid_in_life', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Ailment\s+Damage\s+but\s+you\s+can\s+only\s+deal\s+damage\s+with\s+ailments$/i,
    build: (m) => ({ key: 'ailment_damage_all', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Skills\s+Damage\s+added\s+to\s+the\s+Ailments\s+damage$/i,
    build: (m) => ({ key: 'skill_damage_to_ailments', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Reduced\s+Damage\s+Healed\s+Over\s+Time$/i,
    build: (m) => ({ key: 'damage_to_life_replenish_chance', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Chance\s+on\s+hit\s+with\s+Area\s+of\s+Effect\s+Spells\s+to\s+call\s+down\s+a\s+chaos\s+meteor\s+dealing\s+\d+%\s+of\s+the\s+damage\.?\s+The\s+proc\s+chance\s+is\s+lowered\s+to\s+\d+%\s+with\s+non\s+single\s+hitting\s+skills$/i,
    build: (m) => ({ key: 'additional_explosion_chance', value: num(m[1]!) }),
  },
  {
    test: /^Summons\s+explode\s+at\s+\d+%\s+life\s+dealing\s+area\s+of\s+effect\s+fire\s+damage\s+based\s+on\s+their\s+life$/i,
    build: () => ({ key: 'summon_explode_on_low_life', value: 1 }),
  },
  {
    test: /^Summons\s+now\s+explode\s+instantly\s+after\s+coming\s+in\s+contact\s+with\s+a\s+monster$/i,
    build: () => ({ key: 'summon_instant_explode', value: 1 }),
  },
  {
    test: /^Summon\s+projectile\s+chain\s+hits\s+unleash\s+a\s+Void\s+Blast\s+dealing\s+damage\s+around\s+the\s+target$/i,
    build: () => ({ key: 'summon_chain_void_blast', value: 1 }),
  },
  {
    test: /^Life\s+replenish\s+now\s+happens\s+every\s+2\s+seconds\s+with\s+increased\s+power$/i,
    build: () => ({ key: 'life_replenish_more', value: 0 }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Life\s+Replenish$/i,
    build: (m) => ({ key: 'life_replenish_more', value: num(m[1]!) }),
  },
  {
    test: /^Chance\s+on\s+hit\s+to\s+unleash\s+a\s+Sand\s+Ripple\s+dealing\s+damage\s+on\s+a\s+radius\s+around\s+the\s+target$/i,
    build: () => ({ key: 'sand_ripple_chance', value: 0 }),
  },
  {
    test: /^After\s+\d+\s+attacks\s+guardians\s+unleash\s+a\s+Sand\s+Beam$/i,
    build: () => ({ key: 'sand_beam_damage', value: 0 }),
  },
  {
    test: /^Gain\s+\d+%\s+increased\s+total\s+cast\r?\s+rate\s+but\s+over\s+heat\s+after\s+\d+\s+casts\s+causing\s+decreased\s+cast\s+rate\s+but\s+increased\s+total\s+damage$/i,
    build: () => ({ key: 'wizardry_cast_rate', value: 0 }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Total\s+Damage\s+and\s+Decreased\s+Cast\s+Rate\s+from\s+over\s+heat$/i,
    build: (m) => ({ key: 'enhanced_damage_more', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Increased\s+Attack\s+Speed\s+&\s+Damage\s+and\s+Increased\s+Damage\s+Reduction\s+&\s+All\s+Resistances$/i,
    build: (m) => ({ key: 'increased_attack_speed', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+When\s+at\s+full\s+Life\s+gain\s+increased\s+total\s+Attack\s+Speed\s+and\s+Damage\s+Dealt\s+but\s+also\s+decreased\s+Damage\s+Reduction\s+and\s+All\s+Resistances$/i,
    build: (m) => ({ key: 'attack_speed_full_life', value: num(m[1]!) }),
  },
  {
    test: /^([+\-\d.]+)%\s+Total\s+Summon\s+Projectile\s+Damage$/i,
    build: (m) => ({ key: 'summon_projectile_damage_more', value: num(m[1]!) }),
  },
]

export interface ParsedConversion {
  kind: 'convert'
  fromKey: string
  fromKind: 'stat' | 'attribute'
  toKey: string
  toKind: 'stat' | 'attribute'
  pct: number
}

export type DisableTarget = 'life_replenish'

export interface ParsedDisable {
  kind: 'disable'
  target: DisableTarget
}

export type ParsedMeta = ParsedConversion | ParsedDisable

interface ConversionRule {
  test: RegExp
  build: (m: RegExpMatchArray) => ParsedConversion | null
}

const CONVERSION_TARGET_STATS: Record<string, string> = {
  'magic skill damage': 'magic_skill_damage',
  'damage return': 'damage_return',
  'increased maximum life': 'increased_life',
  'maximum life': 'life',
  'maximum mana': 'mana',
  'physical damage': 'additive_physical_damage',
  'attack damage': 'attack_damage',
  'ranged physical damage': 'ranged_physical_per_500_mana',
  'increased life': 'increased_life',
  'increased damage': 'enhanced_damage',
}

interface DisableRule {
  test: RegExp
  target: DisableTarget
}

const CONVERSION_RULES: ConversionRule[] = [
  {
    test: /^([+\-\d.]+)%\s+of\s+(?:your\s+)?Increased\s+Attack\s+Speed\s+is\s+added\s+as\s+Magic\s+Skill\s+Damage$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'increased_attack_speed',
      fromKind: 'stat',
      toKey: 'magic_skill_damage',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: new RegExp(
      `^([+\\-\\d.]+)%\\s+of\\s+(?:your\\s+)?(${ELEMENT_RE})\\s+Resistance\\s+is\\s+converted\\s+to\\s+Increased\\s+\\2\\s+Skill\\s+Damage$`,
      'i',
    ),
    build: (m) => {
      const element = m[2]!.toLowerCase() as Element
      return {
        kind: 'convert',
        fromKey: `${element}_resistance`,
        fromKind: 'stat',
        toKey: `${element}_skill_damage`,
        toKind: 'stat',
        pct: num(m[1]!),
      }
    },
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+(strength|dexterity|intelligence|energy|vitality|armor)\s+(?:converted\s+to|(?:is\s+)?added\s+as)\s+(.+)$/i,
    build: (m) => {
      const target = m[3]!.trim().toLowerCase()
      const targetKey = CONVERSION_TARGET_STATS[target]
      if (!targetKey) return null
      return {
        kind: 'convert',
        fromKey: m[2]!.toLowerCase(),
        fromKind: 'attribute',
        toKey: targetKey,
        toKind: 'stat',
        pct: num(m[1]!),
      }
    },
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+(?:your\s+)?Defense\s+is\s+converted\s+to\s+life$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'defense',
      fromKind: 'stat',
      toKey: 'life',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Resistances\s+converted\s+to\s+Life$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'all_resistances',
      fromKind: 'stat',
      toKey: 'life',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+All\s+Resistances\s+over\s+the\s+cap\s+converted\s+to\s+life$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'all_resistances',
      fromKind: 'stat',
      toKey: 'life',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Attack\s+Damage\s+converted\s+to\s+Increased\s+Life$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'attack_damage',
      fromKind: 'stat',
      toKey: 'increased_life',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Attack\s+Damage\s+converted\s+to\s+Increased\s+Life$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'attack_damage',
      fromKind: 'stat',
      toKey: 'increased_life',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+(?:your\s+)?Negative\s+All\s+Resistances\s+are\s+added\s+as\s+Increased\s+Maximum\s+Life$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'all_resistances',
      fromKind: 'stat',
      toKey: 'increased_life',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Negative\s+All\s+Resistances\s+added\s+as\s+increased\s+damage$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'all_resistances',
      fromKind: 'stat',
      toKey: 'enhanced_damage',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Maximum\s+Life\s+added\s+as\s+Maximum\s+Mana$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'life',
      fromKind: 'stat',
      toKey: 'mana',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Maximum\s+Mana\s+added\s+as\s+Maximum\s+life$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'mana',
      fromKind: 'stat',
      toKey: 'life',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Increased\s+Maximum\s+Mana\s+added\s+as\s+Magic\s+Skill\s+Damage$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'increased_mana',
      fromKind: 'stat',
      toKey: 'magic_skill_damage',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+(?:your\s+)?Increased\s+Movement\s+Speed\s+converted\s+to\s+Attack\s+Damage$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'movement_speed',
      fromKind: 'stat',
      toKey: 'attack_damage',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Energy\s+is\s+added\s+as\s+Ranged\s+Physical\s+Damage$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'energy',
      fromKind: 'attribute',
      toKey: 'ranged_physical_per_500_mana',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Area\s+of\s+Effect\s+Radius\s+converted\s+to\s+Area\s+of\s+Effect\s+Damage$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'area_of_effect',
      fromKind: 'stat',
      toKey: 'spell_aoe_damage',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Explosion\s+Area\s+of\s+Effect\s+radius\s+converted\s+to\s+Explosion\s+Damage$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'explosion_aoe',
      fromKind: 'stat',
      toKey: 'explosion_damage',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Dexterity\s+Converted\s+to\s+Ranged\s+Projectile\s+Damage$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'dexterity',
      fromKind: 'attribute',
      toKey: 'ranged_projectile_damage',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+Strength\s+converted\s+to\s+weapon\s+damage\s+when\s+Unarmed,\s+Strength\s+no\s+longer\s+provides\s+attack\s+damage\.?$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'strength',
      fromKind: 'attribute',
      toKey: 'str_to_unarmed_damage',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+of\s+physical\s+damage\s+as\s+Arrow\s+Damage$/i,
    build: (m) => ({
      kind: 'convert',
      fromKey: 'additive_physical_damage',
      fromKind: 'stat',
      toKey: 'physical_to_arrow_damage',
      toKind: 'stat',
      pct: num(m[1]!),
    }),
  },
  {
    test: /^([+\-\d.]+)%\s+to\s+Enhanced\s+Damage\s+when\s+using\s+(Bow|Throwing\s+Weapon|Gun|Axe)$/i,
    build: (m) => {
      const weapon = m[2]!.toLowerCase()
      const targetKey =
        weapon === 'bow'
          ? 'damage_with_bow'
          : weapon === 'gun'
          ? 'damage_with_gun'
          : weapon === 'axe'
          ? 'damage_with_axe'
          : 'damage_with_throwing'
      return {
        kind: 'convert',
        fromKey: 'enhanced_damage',
        fromKind: 'stat',
        toKey: targetKey,
        toKind: 'stat',
        pct: num(m[1]!),
      }
    },
  },
  {
    test: new RegExp(
      `^([+\\-\\d.]+)%\\s+of\\s+Physical\\s+Damage\\s+converted\\s+to\\s+(${ELEMENT_RE})$`,
      'i',
    ),
    build: (m) => {
      const element = m[2]!.toLowerCase() as Element
      return {
        kind: 'convert',
        fromKey: 'additive_physical_damage',
        fromKind: 'stat',
        toKey: `physical_to_${element}`,
        toKind: 'stat',
        pct: num(m[1]!),
      }
    },
  },
]

const DISABLE_RULES: DisableRule[] = [
  {
    test: /^You\s+cannot\s+regenerate\s+life\s+from\s+life\s+replenish\s+anymore$/i,
    target: 'life_replenish',
  },
]

const META_CACHE = new Map<string, ParsedMeta | null>()

function parseTreeNodeMeta(line: string): ParsedMeta | null {
  const trimmed = line.trim()
  const cached = META_CACHE.get(trimmed)
  if (cached !== undefined) return cached
  for (const rule of CONVERSION_RULES) {
    const m = trimmed.match(rule.test)
    if (m) {
      const out = rule.build(m)
      if (out && Number.isFinite(out.pct)) {
        META_CACHE.set(trimmed, out)
        return out
      }
    }
  }
  for (const rule of DISABLE_RULES) {
    if (rule.test.test(trimmed)) {
      const out: ParsedDisable = { kind: 'disable', target: rule.target }
      META_CACHE.set(trimmed, out)
      return out
    }
  }
  META_CACHE.set(trimmed, null)
  return null
}

const PARSE_CACHE = new Map<string, ParsedMod | null>()
const RECOGNIZED_NO_STAT = new Set<string>()

function parseTreeNodeMod(line: string): ParsedMod | null {
  const trimmed = line.trim()
  const cached = PARSE_CACHE.get(trimmed)
  if (cached !== undefined) return cached
  const { base, selfCondition } = stripSelfCondition(trimmed)
  const candidates = [base]
  const stripped = stripWeaponContext(base)
  if (stripped !== base) candidates.push(stripped)
  for (const candidate of candidates) {
    for (const rule of RULES) {
      const m = candidate.match(rule.test)
      if (m) {
        const built = rule.build(m)
        if (built === null) {
          RECOGNIZED_NO_STAT.add(trimmed)
          PARSE_CACHE.set(trimmed, null)
          return null
        }
        if (built && Number.isFinite(built.value)) {
          const out: ParsedMod = selfCondition
            ? { ...built, selfCondition }
            : built
          PARSE_CACHE.set(trimmed, out)
          return out
        }
      }
    }
  }
  PARSE_CACHE.set(trimmed, null)
  return null
}

function isRecognizedTreeLine(line: string): boolean {
  const trimmed = line.trim()
  if (RECOGNIZED_NO_STAT.has(trimmed)) return true
  parseTreeNodeMod(trimmed)
  return RECOGNIZED_NO_STAT.has(trimmed)
}

export interface NodeModBreakdown {
  parsed: { line: string; mod: ParsedMod | ParsedMeta }[]
  unsupported: string[]
}

export function classifyNodeLines(lines: string[]): NodeModBreakdown {
  const parsed: NodeModBreakdown['parsed'] = []
  const unsupported: string[] = []
  for (const line of lines) {
    const mod = parseTreeNodeMod(line)
    if (mod) {
      parsed.push({ line, mod })
      continue
    }
    if (isRecognizedTreeLine(line)) continue
    const meta = parseTreeNodeMeta(line)
    if (meta) {
      parsed.push({ line, mod: meta })
      continue
    }
    unsupported.push(line)
  }
  return { parsed, unsupported }
}
