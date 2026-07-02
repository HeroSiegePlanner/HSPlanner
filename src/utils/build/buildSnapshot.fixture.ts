import { defaultEnemyResistances, type BuildSnapshot } from './shareBuild'

export function makeSnapshot(overrides: Partial<BuildSnapshot> = {}): BuildSnapshot {
  return {
    classId: 'amazon',
    level: 5,
    allocated: { strength: 0, dexterity: 0, intelligence: 0, energy: 0, vitality: 0, armor: 0 },
    inventory: {},
    skillRanks: {},
    subskillRanks: {},
    allocatedTreeNodes: new Set<number>(),
    activeSkillIds: [],
    activeAuraId: null,
    activeBuffs: {},
    enemyConditions: {},
    playerConditions: {},
    skillProjectiles: {},
    enemyResistances: defaultEnemyResistances(),
    procToggles: {},
    disabledPotions: {},
    killsPerSec: 1,
    customStats: [],
    treeSocketed: {},
    allocatedEtherNodes: new Set<number>(),
    mercClassId: null,
    mercSkillRanks: {},
    mercInventory: {},
    mercDisabledAuras: {},
    ...overrides,
  }
}
