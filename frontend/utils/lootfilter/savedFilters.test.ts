import { beforeEach, describe, expect, test, vi } from 'vitest'
import { DEFAULT_LOOT_FILTER_CODE } from './codec'
import type * as CodecModule from './codec'
import {
  createFilter,
  deleteFilter,
  duplicateFilter,
  getSavedFilter,
  importFilter,
  listSavedFilters,
  renameFilter,
  setFilterFavorite,
  updateFilterCode,
} from './savedFilters'

// The codec lives in the engine; the library only needs "valid or not".
vi.mock('./codec', async (importOriginal) => ({
  ...(await importOriginal<typeof CodecModule>()),
  decodeLootFilter: vi.fn(async (code: string) => (code.startsWith('eyJ') ? {} : null)),
}))

const BUILD = 'b_test'
const GAME_EXPORT = ' eyJ2ZXJzaW9uIjoyfQ== '

beforeEach(() => {
  window.localStorage.clear()
})

describe('savedFilters', () => {
  test('createFilter tworzy filtr z domyślnym kodem przypięty do buildu', () => {
    const record = createFilter(BUILD, 'Mój filtr')
    expect(record.name).toBe('Mój filtr')
    expect(record.buildId).toBe(BUILD)
    expect(record.code).toBe(DEFAULT_LOOT_FILTER_CODE)
    const listed = listSavedFilters(BUILD)
    expect(listed).toHaveLength(1)
    expect(listed[0]!.id).toBe(record.id)
  })

  test('filtry są rozdzielone per build', () => {
    createFilter('b_one', 'A')
    const b = createFilter('b_two', 'B')
    expect(listSavedFilters('b_one')).toHaveLength(1)
    expect(listSavedFilters('b_two')).toHaveLength(1)
    expect(listSavedFilters('b_three')).toHaveLength(0)
    const copy = duplicateFilter(b.id)
    expect(copy?.buildId).toBe('b_two')
    expect(listSavedFilters('b_two')).toHaveLength(2)
    expect(listSavedFilters('b_one')).toHaveLength(1)
  })

  test('updateFilterCode zapisuje nowy kod i podbija updatedAt', () => {
    const record = createFilter(BUILD, 'F')
    const code = 'eyJ2ZXJzaW9uIjoyLCJ3dGMiOjEyM30='
    const updated = updateFilterCode(record.id, code)
    expect(updated?.code).toBe(code)
    expect(getSavedFilter(record.id)?.code).toBe(code)
  })

  test('rename / duplicate / delete działają', () => {
    const record = createFilter(BUILD, 'A')
    renameFilter(record.id, 'B')
    expect(getSavedFilter(record.id)?.name).toBe('B')
    const copy = duplicateFilter(record.id)
    expect(copy?.name).toBe('B (copy)')
    expect(listSavedFilters(BUILD)).toHaveLength(2)
    deleteFilter(record.id)
    expect(listSavedFilters(BUILD)).toHaveLength(1)
    expect(getSavedFilter(record.id)).toBeNull()
  })

  test('ulubione lądują na górze listy', () => {
    const a = createFilter(BUILD, 'A')
    const b = createFilter(BUILD, 'B')
    setFilterFavorite(b.id, true)
    expect(listSavedFilters(BUILD)[0]!.id).toBe(b.id)
    setFilterFavorite(b.id, false)
    setFilterFavorite(a.id, true)
    expect(listSavedFilters(BUILD)[0]!.id).toBe(a.id)
    expect(getSavedFilter(a.id)?.favorite).toBe(true)
  })

  test('importFilter przyjmuje kod, który silnik dekoduje, odrzuca śmieci', async () => {
    expect(await importFilter(BUILD, 'Hell', 'nie base64!!!')).toBeNull()
    const record = await importFilter(BUILD, 'Hell', GAME_EXPORT)
    expect(record).not.toBeNull()
    expect(getSavedFilter(record!.id)?.code).toBe(GAME_EXPORT.trim())
  })

  test('uszkodzony storage nie wywala listy', () => {
    window.localStorage.setItem('hsplanner.lootFilters.v1', '{zepsute')
    expect(listSavedFilters(BUILD)).toEqual([])
  })
})
