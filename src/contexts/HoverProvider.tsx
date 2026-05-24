import { useCallback, useMemo, useState, type ReactNode } from 'react'
import { HoverContext, type HoverPreview } from './HoverContext'

// Single source of truth for what the user is hovering across the whole app.
// Kept root-level (wrapped around <App>) rather than per-view because the
// consumer (left sidebar) lives outside the views that emit hover events.
export function HoverProvider({ children }: { children: ReactNode }) {
  const [preview, setPreviewState] = useState<HoverPreview>(null)
  const setPreview = useCallback((next: HoverPreview) => {
    setPreviewState(next)
  }, [])
  const value = useMemo(() => ({ preview, setPreview }), [preview, setPreview])
  return <HoverContext.Provider value={value}>{children}</HoverContext.Provider>
}
