import { invoke } from '@tauri-apps/api/core'
import { activeSeasonId } from '@data'
import type { Inventory } from '../../types'

export interface BuildFilterStats {
  statIds: number[]
  // Gear stats with no counterpart in the game's filter list.
  unmatched: number
}

export async function buildFilterStats(inventory: Inventory): Promise<BuildFilterStats> {
  return await invoke<BuildFilterStats>('lootfilter_build_stats', {
    inventory,
    season: activeSeasonId,
  })
}

export async function lootFilterCodeForStats(
  statIds: number[],
  hideRest: boolean,
): Promise<string> {
  return await invoke<string>('lootfilter_code_for_stats', { statIds, hideRest })
}
