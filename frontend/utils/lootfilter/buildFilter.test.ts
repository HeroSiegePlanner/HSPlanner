import { describe, expect, test } from 'vitest'
import type { Inventory } from '../../types'
import { decodeLootFilter, encodeLootFilter } from './codec'
import { FILTER_STATS, FILTER_STAT_BY_ID } from './constants'
import {
  buildFilterForStats,
  collectBuildStats,
  filterStatIdsFor,
} from './buildFilter'

const nameOf = (id: number) => FILTER_STAT_BY_ID.get(id)?.name
const idOf = (name: string) => FILTER_STATS.find((s) => s.name === name)!.id

function item(affixIds: string[]) {
  return {
    baseId: 'whatever',
    affixes: affixIds.map((affixId) => ({ affixId, tier: 1, roll: 0 })),
    socketCount: 0,
    socketed: [],
    socketTypes: [],
  }
}

describe('mapowanie statów buildu na afiksy filtra', () => {
  test('trafia w afiks o tej samej nazwie co stat', () => {
    const ids = filterStatIdsFor([{ statKey: 'fire_resistance', format: 'percent' }])
    expect(ids.map(nameOf)).toEqual(['Fire Resistance'])
  })

  test('format rozstrzyga wariant procentowy — bez niego trafiłby zły afiks', () => {
    expect(filterStatIdsFor([{ statKey: 'life', format: 'flat' }]).map(nameOf)).toEqual(
      ['Life'],
    )
    expect(
      filterStatIdsFor([{ statKey: 'increased_life', format: 'percent' }]).map(nameOf),
    ).toEqual(['Life %'])
  })

  test('alias rozdziela flat i procentowe obrażenia od żywiołów', () => {
    expect(
      filterStatIdsFor([{ statKey: 'fire_skill_damage', format: 'flat' }]).map(nameOf),
    ).toEqual(['Flat Fire Skill Damage'])
    expect(
      filterStatIdsFor([{ statKey: 'fire_skill_damage', format: 'percent' }]).map(
        nameOf,
      ),
    ).toEqual(['Fire Skill Damage %'])
  })

  test('stat bez odpowiednika w filtrze jest pomijany, nie zgadywany', () => {
    expect(filterStatIdsFor([{ statKey: 'nie_ma_takiego', format: 'flat' }])).toEqual(
      [],
    )
  })

  test('duplikaty statów dają jedno ID', () => {
    const ids = filterStatIdsFor([
      { statKey: 'fire_resistance', format: 'percent' },
      { statKey: 'fire_resistance', format: 'percent' },
    ])
    expect(ids).toHaveLength(1)
  })
})

describe('collectBuildStats', () => {
  test('zbiera afiksy z założonych przedmiotów i deduplikuje', () => {
    const inventory = {
      helmet: item([
        '7_15_to_fire_resistance_t1_firecloaking',
        '15_30_to_life_t1_bear',
      ]),
      armor: item(['7_15_to_fire_resistance_t2_fireproof']),
    } as unknown as Inventory
    const stats = collectBuildStats(inventory)
    expect(stats).toEqual(
      expect.arrayContaining([
        { statKey: 'fire_resistance', format: 'percent' },
        { statKey: 'life', format: 'flat' },
      ]),
    )
    expect(stats).toHaveLength(2)
  })

  test('pusty ekwipunek nie daje statów', () => {
    expect(collectBuildStats({})).toEqual([])
  })

  test('bierze staty unikatu z implicit — endgame nie nosi losowych afiksów', () => {
    const inventory = {
      helmet: { ...item([]), baseId: 'helmet_angelic_lucifers_crown' },
    } as unknown as Inventory
    const keys = collectBuildStats(inventory).map((s) => s.statKey)
    expect(keys).toEqual(
      expect.arrayContaining(['enhanced_defense', 'all_skills', 'all_resistances']),
    )
  })

  test('format statu z implicit bierze się z katalogu gry', () => {
    const inventory = {
      helmet: { ...item([]), baseId: 'helmet_angelic_mask_of_the_celestial' },
    } as unknown as Inventory
    const stats = collectBuildStats(inventory)
    expect(stats).toEqual(
      expect.arrayContaining([
        { statKey: 'life', format: 'flat' },
        { statKey: 'increased_life', format: 'percent' },
      ]),
    )
    const names = filterStatIdsFor(stats).map(nameOf)
    expect(names).toEqual(expect.arrayContaining(['Life', 'Life %']))
  })
})

describe('buildFilterForStats', () => {
  const ids = [idOf('Life'), idOf('Fire Resistance')]

  test('podświetla wskazane afiksy na każdym typie i tierze', () => {
    const filter = buildFilterForStats(ids)
    for (const type of Object.values(filter.types)) {
      for (const tier of type.tiers) {
        expect(tier.highlighted).toEqual(expect.arrayContaining(ids))
        expect(tier.hidden).toEqual([])
      }
    }
  })

  test('hideRest ukrywa resztę, nigdy podświetlonych', () => {
    const filter = buildFilterForStats(ids, { hideRest: true })
    const tier = filter.types[0]!.tiers[0]!
    expect(tier.hidden).toHaveLength(FILTER_STATS.length - ids.length)
    for (const id of ids) expect(tier.hidden).not.toContain(id)
    expect(tier.highlighted).toEqual(expect.arrayContaining(ids))
  })

  test('przechodzi round-trip przez format gry', () => {
    const filter = buildFilterForStats(ids, { hideRest: true })
    const decoded = decodeLootFilter(encodeLootFilter(filter))
    expect(decoded).not.toBeNull()
    expect([...decoded!.types[0]!.tiers[0]!.highlighted].sort()).toEqual(
      [...ids].sort(),
    )
  })

  test('brak statów daje czysty filtr domyślny', () => {
    const filter = buildFilterForStats([])
    expect(filter.types[0]!.tiers[0]!.highlighted).toEqual([])
    expect(filter.types[0]!.tiers[0]!.hidden).toEqual([])
  })
})
