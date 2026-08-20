import { createContext, useContext } from 'react'
import type { SlotKey } from '../types'

export type HoverPreview =
  | { kind: 'gear'; slot: SlotKey; baseId: string }
  | { kind: 'tree'; nodeId: number }
  | null

export type SetHoverPreview = (next: HoverPreview) => void

export const HoverContext = createContext<SetHoverPreview | null>(null)

export function useSetHoverPreview(): SetHoverPreview {
  const ctx = useContext(HoverContext)
  if (!ctx) {
    throw new Error('useSetHoverPreview must be used inside <HoverProvider>')
  }
  return ctx
}
