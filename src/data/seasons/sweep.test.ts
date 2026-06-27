import { describe, expect, it } from 'vitest'
import { SEASONS } from './registry'
import { loadSeasonPatchSet } from './load'
import {
  applyGameConfigPatch,
  applyListPatch,
  applyRecordMergePatch,
  applyRecordReplacePatch,
  applyTreePatch,
} from './resolve'
import type {
  HeroSiegeTree,
  RecordPatch,
  SeasonPatchSet,
} from './patchTypes'
import affixesJson from '../affixes.json'
import augmentsJson from '../augments.json'
import crystalsJson from '../crystals.json'
import gameConfigJson from '../game-config.json'
import heroSiegeTreeJson from '../hero-siege-tree.json'
import itemGrantedSkillsJson from '../item-granted-skills.json'
import nodeIconsJson from '../node-icons.json'
import runewordsJson from '../runewords.json'
import setsJson from '../sets.json'
import starScalingJson from '../star-scaling.json'
import treeNodesJson from '../tree-nodes.json'

type Rec = Record<string, unknown>

const itemModules = import.meta.glob<{ default: Rec[] }>('../items/*.json', {
  eager: true,
})
const gemModules = import.meta.glob<{ default: Rec[] }>('../gems/*.json', {
  eager: true,
})
const runeModules = import.meta.glob<{ default: Rec[] }>('../runes/*.json', {
  eager: true,
})
const skillModules = import.meta.glob<{ default: Rec[] }>('../skills/*.json', {
  eager: true,
})
const classModules = import.meta.glob<{ default: Rec }>('../classes/*.json', {
  eager: true,
})

function collectFlat(modules: Record<string, { default: Rec[] }>): Rec[] {
  return Object.values(modules).flatMap((m) => m.default)
}

function collectScalar(modules: Record<string, { default: Rec }>): Rec[] {
  return Object.values(modules).map((m) => m.default)
}

const cases: ReadonlyArray<{
  name: string
  apply: (p: SeasonPatchSet) => { errors: string[] }
}> = [
  {
    name: 'affixes',
    apply: (p) => applyListPatch(affixesJson as Rec[], p.affixes, 'affixes'),
  },
  {
    name: 'crystals',
    apply: (p) => applyListPatch(crystalsJson as Rec[], p.crystals, 'crystals'),
  },
  {
    name: 'augments',
    apply: (p) => applyListPatch(augmentsJson as Rec[], p.augments, 'augments'),
  },
  {
    name: 'runewords',
    apply: (p) =>
      applyListPatch(runewordsJson as Rec[], p.runewords, 'runewords'),
  },
  {
    name: 'sets',
    apply: (p) => applyListPatch(setsJson as Rec[], p.sets, 'sets'),
  },
  {
    name: 'items',
    apply: (p) => applyListPatch(collectFlat(itemModules), p.items, 'items'),
  },
  {
    name: 'gems',
    apply: (p) => applyListPatch(collectFlat(gemModules), p.gems, 'gems'),
  },
  {
    name: 'runes',
    apply: (p) => applyListPatch(collectFlat(runeModules), p.runes, 'runes'),
  },
  {
    name: 'skills',
    apply: (p) => applyListPatch(collectFlat(skillModules), p.skills, 'skills'),
  },
  {
    name: 'classes',
    apply: (p) =>
      applyListPatch(collectScalar(classModules), p.classes, 'classes'),
  },
  {
    name: 'item-granted-skills',
    apply: (p) =>
      applyListPatch(
        itemGrantedSkillsJson as Rec[],
        p.itemGrantedSkills,
        'item-granted-skills',
        'name',
      ),
  },
  {
    name: 'tree-nodes',
    apply: (p) =>
      applyRecordMergePatch(
        treeNodesJson as Record<string, Rec>,
        p.treeNodes as unknown as RecordPatch<Rec> | undefined,
        'tree-nodes',
      ),
  },
  {
    name: 'node-icons',
    apply: (p) =>
      applyRecordReplacePatch(
        nodeIconsJson as Record<string, string>,
        p.nodeIcons,
        'node-icons',
      ),
  },
  {
    name: 'hero-siege-tree',
    apply: (p) =>
      applyTreePatch(
        heroSiegeTreeJson as HeroSiegeTree,
        p.heroSiegeTree,
        'hero-siege-tree',
      ),
  },
  {
    name: 'game-config',
    apply: (p) =>
      applyGameConfigPatch(gameConfigJson as Rec, p.gameConfig, 'game-config'),
  },
  {
    name: 'star-scaling',
    apply: (p) =>
      applyRecordMergePatch(
        starScalingJson as unknown as Record<string, Rec>,
        p.starScaling,
        'star-scaling',
      ),
  },
]

describe('season sweep', () => {
  for (const season of SEASONS) {
    it(`${season.id}: patches load and apply cleanly against base data`, () => {
      const load = loadSeasonPatchSet(season.id)
      expect(load.errors).toEqual([])
      for (const { name, apply } of cases) {
        const r = apply(load.patches)
        expect(r.errors, `${season.id}/${name}`).toEqual([])
      }
    })
  }
})
