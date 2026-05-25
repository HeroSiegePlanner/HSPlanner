import { createContext, useContext } from 'react'
import type { SlotKey } from '../types'

// `null` when nothing is hovered so downstream consumers can render a prompt instead of fake numbers.
export type HoverPreview =
  | { kind: 'gear'; slot: SlotKey; baseId: string }
  | { kind: 'tree'; nodeId: number }
  | null

export interface HoverContextValue {
  preview: HoverPreview
  setPreview: (next: HoverPreview) => void
}

export const HoverContext = createContext<HoverContextValue | null>(null)

function useHoverContext(): HoverContextValue {
  const ctx = useContext(HoverContext)
  if (!ctx) {
    throw new Error(
      'useHoverPreview / useSetHoverPreview must be used inside <HoverProvider>',
    )
  }
  return ctx
}

export function useHoverPreview(): HoverPreview {
  return useHoverContext().preview
}

export function useSetHoverPreview(): (next: HoverPreview) => void {
  return useHoverContext().setPreview
}
