import { invoke } from '@tauri-apps/api/core'
import { activeSeasonId } from '@data'
import type { EquippedItem } from '../../types'

export interface TooltipLine {
  text: string
  status: 'matched' | 'ignored' | 'warning'
  detail?: string
}

export interface TooltipParseResult {
  baseId: string | null
  equipped: EquippedItem | null
  lines: TooltipLine[]
  errors: string[]
}

// Parsing lives in engine/src/tooltip_parse.rs next to the OCR.
export async function parseTooltipLines(lines: string[]): Promise<TooltipParseResult> {
  return await invoke<TooltipParseResult>('parse_tooltip_lines', {
    lines,
    season: activeSeasonId,
  })
}
