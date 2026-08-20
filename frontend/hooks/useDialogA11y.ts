import { useEffect, useRef, useState, type RefObject } from 'react'

const openDialogs: HTMLElement[] = []

/**
 * Moves focus into the dialog, keeps Tab out of the background (inert), restores
 * focus on close and routes Escape to the topmost dialog only.
 */
export function useDialogA11y(
  panelRef: RefObject<HTMLElement | null>,
  onEscape: () => void,
  inertRoot = true,
) {
  const escapeRef = useRef(onEscape)
  useEffect(() => {
    escapeRef.current = onEscape
  })

  // read during render: an autoFocus child steals activeElement before effects run
  const [restoreTo] = useState(() => document.activeElement as HTMLElement | null)

  useEffect(() => {
    const panel = panelRef.current
    if (!panel) return
    if (!panel.contains(document.activeElement)) panel.focus()

    openDialogs.push(panel)
    // portal=false renders inside #root, so inerting it would disable the dialog too
    const root = inertRoot ? document.getElementById('root') : null
    if (openDialogs.length === 1) root?.toggleAttribute('inert', true)

    // window listener, not a DOM handler: clicking non-focusable content can park focus on <body>
    function onKey(e: KeyboardEvent) {
      if (e.key !== 'Escape') return
      if (openDialogs[openDialogs.length - 1] !== panel) return
      escapeRef.current()
    }
    window.addEventListener('keydown', onKey)

    return () => {
      window.removeEventListener('keydown', onKey)
      openDialogs.splice(openDialogs.indexOf(panel), 1)
      if (openDialogs.length === 0) root?.toggleAttribute('inert', false)
      restoreTo?.focus?.()
    }
  }, [panelRef, inertRoot, restoreTo])
}
