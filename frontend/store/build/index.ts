import { create } from 'zustand'
import { setBridgeErrorListener } from '../../utils/calc/bridge'
import { createCharacterSlice } from './characterSlice'
import { createCombatSlice } from './combatSlice'
import { createEtherSlice } from './etherSlice'
import { createInventorySlice } from './inventorySlice'
import { createMercSlice } from './mercSlice'
import { createSavedBuildsSlice } from './savedBuildsSlice'
import { createSkillsSlice } from './skillsSlice'
import { createStashSlice } from './stashSlice'
import { createTreeSlice } from './treeSlice'
import type { BuildStore } from './types'

export {
  MAX_STARS,
  maxSocketsFor,
  BONUS_SOCKET_MOD_ID,
  RAINBOW_MULTIPLIER,
} from '../itemRules'

export {
  skillPointsFor,
  subskillPointsFor,
  subskillKey,
  subskillSpentFor,
  attrPointsFor,
  finalAttributes,
} from './helpers'

export const useBuild = create<BuildStore>((...a) => ({
  ...createCharacterSlice(...a),
  ...createInventorySlice(...a),
  ...createSkillsSlice(...a),
  ...createStashSlice(...a),
  ...createTreeSlice(...a),
  ...createEtherSlice(...a),
  ...createMercSlice(...a),
  ...createCombatSlice(...a),
  ...createSavedBuildsSlice(...a),
}))

setBridgeErrorListener((err) => {
  useBuild.setState({
    storageError: `Calculation failed: ${err.message}`,
  })
})
