import { useLayoutEffect, useState, type ReactNode, type RefObject } from 'react'
import { createPortal } from 'react-dom'

interface HoverPortalProps {
  anchorRef: RefObject<HTMLDivElement | null>
  children: ReactNode
}

function HoverPortal({ anchorRef, children }: HoverPortalProps) {
  const [pos, setPos] = useState<{ left: number; top: number; maxWidth: number } | null>(null)

  useLayoutEffect(() => {
    function recompute() {
      const anchor = anchorRef.current
      if (!anchor) return
      const rect = anchor.getBoundingClientRect()
      const margin = 12
      const desiredMaxWidth = Math.min(640, rect.left - margin * 2)
      const left = Math.max(margin, rect.left - margin)
      setPos({
        left,
        top: rect.top,
        maxWidth: Math.max(220, desiredMaxWidth),
      })
    }
    recompute()
    const opts = { capture: true, passive: true } as const
    window.addEventListener('resize', recompute)
    window.addEventListener('scroll', recompute, opts)
    return () => {
      window.removeEventListener('resize', recompute)
      window.removeEventListener('scroll', recompute, opts)
    }
  }, [anchorRef])

  if (!pos) return null
  return createPortal(
    <div
      className="fixed z-[1000] pointer-events-none"
      style={{
        left: pos.left,
        top: pos.top,
        transform: 'translateX(-100%)',
        maxWidth: `${pos.maxWidth}px`,
      }}
    >
      {children}
    </div>,
    document.body,
  )
}

export default HoverPortal
