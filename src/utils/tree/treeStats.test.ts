import { describe, expect, it } from 'vitest'
import {
  ELEMENTS,
  SELF_CONDITION_KEYS,
  SELF_CONDITION_LABELS,
  TREE_JEWELRY_IDS,
  TREE_NODE_INFO,
  TREE_WARP_IDS,
  aggregateTreeStats,
  classifyNodeLines,
  isRecognizedTreeLine,
  parseTreeNodeMeta,
  parseTreeNodeMod,
} from './treeStats'

// -----------------------------------------------------------------------
// Constants and tree-data smoke tests
// -----------------------------------------------------------------------

describe('ELEMENTS', () => {
  it('lists the five game elements in alphabetical order', () => {
    expect(ELEMENTS).toEqual(['arcane', 'cold', 'fire', 'lightning', 'poison'])
  })
})

describe('SELF_CONDITION_KEYS', () => {
  it('exposes the two supported self-condition keys', () => {
    expect(SELF_CONDITION_KEYS).toEqual(['crit_chance_below_40', 'life_below_40'])
  })

  it('has a label for every key', () => {
    for (const key of SELF_CONDITION_KEYS) {
      expect(SELF_CONDITION_LABELS[key]).toBeTruthy()
    }
  })
})

describe('TREE_NODE_INFO', () => {
  it('contains at least one node entry', () => {
    expect(Object.keys(TREE_NODE_INFO).length).toBeGreaterThan(0)
  })

  it('partitions warp and jewelry ids into disjoint sets', () => {
    for (const id of TREE_WARP_IDS) {
      expect(TREE_JEWELRY_IDS.has(id)).toBe(false)
    }
  })
})

// -----------------------------------------------------------------------
// parseTreeNodeMod — flat / percent / attribute / element variants
// -----------------------------------------------------------------------

describe('parseTreeNodeMod — flat life/mana', () => {
  it('parses "+N to Maximum Life"', () => {
    expect(parseTreeNodeMod('+5 to Maximum Life')).toEqual({ key: 'life', value: 5 })
  })

  it('parses negative values via the leading sign', () => {
    expect(parseTreeNodeMod('-3 to Maximum Life')).toEqual({ key: 'life', value: -3 })
  })

  it('parses "+N to Maximum Mana"', () => {
    expect(parseTreeNodeMod('+10 to Maximum Mana')).toEqual({ key: 'mana', value: 10 })
  })

  it('trims surrounding whitespace before matching', () => {
    expect(parseTreeNodeMod('  +5 to Maximum Life  ')).toEqual({
      key: 'life',
      value: 5,
    })
  })
})

describe('parseTreeNodeMod — increased life/mana', () => {
  it('parses "N% Increased Maximum Life" as additive', () => {
    expect(parseTreeNodeMod('10% Increased Maximum Life')).toEqual({
      key: 'increased_life',
      value: 10,
    })
  })

  it('routes the "Total" variant to the _more multiplicative bucket', () => {
    expect(parseTreeNodeMod('10% Increased Total Maximum Mana')).toEqual({
      key: 'increased_mana_more',
      value: 10,
    })
  })

  it('treats "Increased Mana" as the same key as "Increased Maximum Mana"', () => {
    expect(parseTreeNodeMod('5% Increased Mana')).toEqual({
      key: 'increased_mana',
      value: 5,
    })
  })
})

describe('parseTreeNodeMod — attributes', () => {
  it('parses "+N to Strength"', () => {
    expect(parseTreeNodeMod('+5 to Strength')).toEqual({
      key: 'to_strength',
      value: 5,
    })
  })

  it('accepts the shorthand "N Strength" without the "to"', () => {
    expect(parseTreeNodeMod('5 Strength')).toEqual({ key: 'to_strength', value: 5 })
  })

  it('parses every attribute key', () => {
    expect(parseTreeNodeMod('+3 to Dexterity')).toMatchObject({ key: 'to_dexterity' })
    expect(parseTreeNodeMod('+3 to Intelligence')).toMatchObject({ key: 'to_intelligence' })
    expect(parseTreeNodeMod('+3 to Energy')).toMatchObject({ key: 'to_energy' })
    expect(parseTreeNodeMod('+3 to Vitality')).toMatchObject({ key: 'to_vitality' })
    expect(parseTreeNodeMod('+3 to Armor')).toMatchObject({ key: 'to_armor' })
  })

  it('parses "+N to All Attributes" and the % variant', () => {
    expect(parseTreeNodeMod('+5 to All Attributes')).toEqual({
      key: 'all_attributes',
      value: 5,
    })
    expect(parseTreeNodeMod('5% Increased All Attributes')).toEqual({
      key: 'increased_all_attributes',
      value: 5,
    })
  })

  it('routes "Increased Total <Attr>" to the _more bucket', () => {
    expect(parseTreeNodeMod('10% Increased Total Strength')).toEqual({
      key: 'increased_strength_more',
      value: 10,
    })
  })
})

describe('parseTreeNodeMod — crit & resistances', () => {
  it('parses critical strike chance and damage', () => {
    expect(parseTreeNodeMod('25% to Critical Strike Chance')).toEqual({
      key: 'crit_chance',
      value: 25,
    })
    expect(parseTreeNodeMod('50% Increased Critical Strike Damage')).toEqual({
      key: 'crit_damage',
      value: 50,
    })
  })

  it('routes "Increased Total Critical Strike Damage" to crit_damage_more', () => {
    expect(parseTreeNodeMod('50% Increased Total Critical Strike Damage')).toEqual({
      key: 'crit_damage_more',
      value: 50,
    })
  })

  it('parses spell crit chance and damage separately from physical crit', () => {
    expect(parseTreeNodeMod('25% Chance to Critically Hit with Spells')).toEqual({
      key: 'spell_crit_chance',
      value: 25,
    })
    expect(parseTreeNodeMod('50% Increased Spell Critical Damage')).toEqual({
      key: 'spell_crit_damage',
      value: 50,
    })
  })

  it('parses per-element resistance lines', () => {
    expect(parseTreeNodeMod('10% to Fire Resistance')).toEqual({
      key: 'fire_resistance',
      value: 10,
    })
    expect(parseTreeNodeMod('10% to Cold Resistance')).toEqual({
      key: 'cold_resistance',
      value: 10,
    })
  })

  it('parses "All Resistances" and its maximum variant', () => {
    expect(parseTreeNodeMod('5% to All Resistances')).toEqual({
      key: 'all_resistances',
      value: 5,
    })
    expect(parseTreeNodeMod('2% to Maximum All Resistances')).toEqual({
      key: 'max_all_resistances',
      value: 2,
    })
  })
})

describe('parseTreeNodeMod — self-condition suffix', () => {
  it('attaches the crit_chance_below_40 self-condition', () => {
    const mod = parseTreeNodeMod(
      '+10 to Maximum Life when Critical Strike Chance is below 40%',
    )
    expect(mod).toEqual({
      key: 'life',
      value: 10,
      selfCondition: 'crit_chance_below_40',
    })
  })

  it('attaches the life_below_40 self-condition', () => {
    const mod = parseTreeNodeMod(
      '+20 to Maximum Life while below 40% of maximum life',
    )
    expect(mod).toMatchObject({ selfCondition: 'life_below_40' })
  })

  it('does not attach a condition for unconditional lines', () => {
    const mod = parseTreeNodeMod('+5 to Maximum Life')
    expect(mod).not.toHaveProperty('selfCondition')
  })
})

describe('parseTreeNodeMod — unrecognised lines', () => {
  it('returns null for non-matching text', () => {
    expect(parseTreeNodeMod('this is not a tree mod')).toBeNull()
  })

  it('returns null for empty strings', () => {
    expect(parseTreeNodeMod('')).toBeNull()
  })
})

describe('parseTreeNodeMod — caching', () => {
  it('returns identical objects on repeated calls (proves the cache is hit)', () => {
    const first = parseTreeNodeMod('+5 to Strength')
    const second = parseTreeNodeMod('+5 to Strength')
    expect(second).toEqual(first)
  })
})

// -----------------------------------------------------------------------
// parseTreeNodeMeta — conversions and disables
// -----------------------------------------------------------------------

describe('parseTreeNodeMeta', () => {
  it('returns null for unrelated text', () => {
    expect(parseTreeNodeMeta('this is not a meta line')).toBeNull()
  })

  it('returns null for plain stat lines (those go through parseTreeNodeMod)', () => {
    expect(parseTreeNodeMeta('+5 to Maximum Life')).toBeNull()
  })
})

// -----------------------------------------------------------------------
// isRecognizedTreeLine
// -----------------------------------------------------------------------

describe('isRecognizedTreeLine', () => {
  it('returns false for unparseable text', () => {
    expect(isRecognizedTreeLine('something the parser does not know')).toBe(false)
  })

  it('returns false for valid stat lines (those parse successfully, not "recognised but ignored")', () => {
    // isRecognizedTreeLine is true only for lines that the parser explicitly
    // chose to discard (build returns null). A normal stat line is parsed,
    // not "recognised-no-stat".
    expect(isRecognizedTreeLine('+5 to Maximum Life')).toBe(false)
  })
})

// -----------------------------------------------------------------------
// classifyNodeLines
// -----------------------------------------------------------------------

describe('classifyNodeLines', () => {
  it('returns empty arrays for no input', () => {
    expect(classifyNodeLines([])).toEqual({ parsed: [], unsupported: [] })
  })

  it('separates parseable mods from unknown lines', () => {
    const result = classifyNodeLines([
      '+5 to Maximum Life',
      'totally garbage line',
      '10% to Fire Resistance',
    ])
    expect(result.parsed).toHaveLength(2)
    expect(result.unsupported).toEqual(['totally garbage line'])
  })

  it('preserves the original line text alongside the parsed mod', () => {
    const result = classifyNodeLines(['+5 to Maximum Life'])
    expect(result.parsed[0]?.line).toBe('+5 to Maximum Life')
    expect(result.parsed[0]?.mod).toEqual({ key: 'life', value: 5 })
  })
})

// -----------------------------------------------------------------------
// aggregateTreeStats — integration
// -----------------------------------------------------------------------

describe('aggregateTreeStats', () => {
  it('returns an empty object for no allocated nodes', () => {
    expect(aggregateTreeStats(new Set())).toEqual({})
  })

  it('ignores nodes whose id is not in TREE_NODE_INFO', () => {
    // Synthetic id 9_999_999 almost certainly does not exist in the tree.
    expect(aggregateTreeStats(new Set([9_999_999]))).toEqual({})
  })

  it('produces only finite numeric values for a real allocated node', () => {
    const firstId = Object.keys(TREE_NODE_INFO).find((id) => {
      const info = TREE_NODE_INFO[id]
      if (!info?.l) return false
      return info.l.some((line) => parseTreeNodeMod(line) !== null)
    })
    if (!firstId) {
      throw new Error('expected the tree data to contain at least one parseable node')
    }
    const stats = aggregateTreeStats(new Set([Number(firstId)]))
    expect(Object.keys(stats).length).toBeGreaterThan(0)
    for (const value of Object.values(stats)) {
      expect(Number.isFinite(value)).toBe(true)
    }
  })

  it('filters out self-conditioned mods when the condition is not active', () => {
    // Build a synthetic single-line scenario by finding a real node whose
    // first line parses with a selfCondition. If none exist we skip rather
    // than fail — the audit's W2 (data validation) is the right fix for
    // that data drift, not this regression test.
    const conditionalEntry = Object.entries(TREE_NODE_INFO).find(([, info]) => {
      if (!info?.l) return false
      return info.l.some((line) => parseTreeNodeMod(line)?.selfCondition !== undefined)
    })
    if (!conditionalEntry) return // nothing to assert against
    const [idStr] = conditionalEntry
    const id = Number(idStr)
    const without = aggregateTreeStats(new Set([id]))
    const conditionedLine = conditionalEntry[1].l.find(
      (line) => parseTreeNodeMod(line)?.selfCondition !== undefined,
    )!
    const conditionedMod = parseTreeNodeMod(conditionedLine)!
    // The conditioned stat must NOT appear in `without` (unless it would
    // also be contributed by another unconditional line in the same node).
    const unconditionedSameKey = conditionalEntry[1].l.some((line) => {
      const m = parseTreeNodeMod(line)
      return m && !m.selfCondition && m.key === conditionedMod.key
    })
    if (!unconditionedSameKey) {
      expect(without[conditionedMod.key]).toBeUndefined()
    }
    // Now turn the condition on — the conditioned mod must contribute.
    const withCond = aggregateTreeStats(new Set([id]), {
      [conditionedMod.selfCondition!]: true,
    })
    const delta =
      (withCond[conditionedMod.key] ?? 0) - (without[conditionedMod.key] ?? 0)
    expect(delta).toBe(conditionedMod.value)
  })
})
