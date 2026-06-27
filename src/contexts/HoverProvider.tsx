import { useState, type ReactNode } from 'react'
import { HoverContext, type HoverPreview } from './HoverContext'

export function HoverProvider({ children }: { children: ReactNode }) {
  const [, setPreview] = useState<HoverPreview>(null)
  return <HoverContext.Provider value={setPreview}>{children}</HoverContext.Provider>
}
