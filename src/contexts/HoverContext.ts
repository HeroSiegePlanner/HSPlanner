import { createContext, useContext } from 'react'
import type { SlotKey } from '../types'

// What's currently being hovered for "preview" purposes (gear item in a
// picker, or a tree node). Stays null when nothing is being hovered, so
// downstream consumers (LeftStatsPanel delta section in phase B) can render
// an "Hover an item to compare" prompt instead of fake numbers.
export type HoverPreview =
  | { kind: 'gear'; slot: SlotKey; baseId: string }
  | { kind: 'tree'; nodeId: number }
  | null

export interface HoverContextValue {
  preview: HoverPreview
  setPreview: (next: HoverPreview) => void
}

// Internal — the actual React context. The provider that fills it lives in
// HoverProvider.tsx; consumers use the hooks below.
export const HoverContext = createContext<HoverContextValue | null>(null)

function useHoverContext(): HoverContextValue {
  // Throws an explicit error when used outside <HoverProvider> — cheaper
  // than returning a default and chasing a silent miss later.
  const ctx = useContext(HoverContext)
  if (!ctx) {
    throw new Error(
      'useHoverPreview / useSetHoverPreview must be used inside <HoverProvider>',
    )
  }
  return ctx
}

// Read-side hook: returns the currently hovered target (or null). Re-renders
// the caller whenever the preview changes.
export function useHoverPreview(): HoverPreview {
  return useHoverContext().preview
}

// Write-side hook: returns the setter so callers can wire it directly into
// `onMouseEnter` / `onMouseLeave`. The provider memoises it once so closures
// stay stable across renders.
export function useSetHoverPreview(): (next: HoverPreview) => void {
  return useHoverContext().setPreview
}
