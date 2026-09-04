import type { ReactNode } from 'react'
import { UpdateContext, useUpdateCheck } from './useUpdateCheck'

/** Holds the one update check the whole app shares. */
export default function UpdateProvider({ children }: { children: ReactNode }) {
  const value = useUpdateCheck()
  return <UpdateContext value={value}>{children}</UpdateContext>
}
