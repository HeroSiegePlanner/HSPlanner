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
    killsPerSec: 1,
    customStats: [],
    treeSocketed: {},
    ...overrides,
  }
}
