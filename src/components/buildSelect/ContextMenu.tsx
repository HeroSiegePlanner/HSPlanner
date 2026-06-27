import { useEffect } from 'react'
import { createPortal } from 'react-dom'
import { motion } from 'motion/react'
import { panelVariants } from '../../lib/motion'

export interface ContextMenuItem {
  label: string
  onClick: () => void
  danger?: boolean
  disabled?: boolean
  kbd?: string
  separatorBefore?: boolean
}

interface ContextMenuProps {
  x: number
  y: number
  header?: string
  items: ContextMenuItem[]
  onClose: () => void
}

const MENU_WIDTH = 220
const ITEM_HEIGHT = 30

export function ContextMenu({ x, y, header, items, onClose }: ContextMenuProps) {
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    window.addEventListener('mousedown', onClose)
    window.addEventListener('resize', onClose)
    return () => {
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('mousedown', onClose)
      window.removeEventListener('resize', onClose)
    }
  }, [onClose])

  const estHeight = (header ? 26 : 0) + items.length * ITEM_HEIGHT + 12
  const left = Math.min(x, window.innerWidth - MENU_WIDTH - 8)
  const top = Math.min(y, window.innerHeight - estHeight - 8)

  return createPortal(
    <motion.div
      role="menu"
      onMouseDown={(e) => e.stopPropagation()}
      variants={panelVariants}
      initial="initial"
      animate="animate"
      className="fixed z-200 flex flex-col overflow-hidden rounded-sm border border-accent-deep/60 py-1"
      style={{
        left,
        top,
        minWidth: MENU_WIDTH,
        background: 'var(--color-panel)',
        boxShadow:
          '0 12px 32px rgba(0,0,0,0.65), inset 0 1px 0 rgba(255,255,255,0.03)',
      }}
    >
      {header && (
        <div className="truncate border-b border-border px-3 py-1.5 font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
          {header}
        </div>
      )}
      {items.map((item, i) => (
        <div key={`${item.label}-${i}`}>
          {item.separatorBefore && <div className="my-1 h-px bg-border" />}
          <button
            type="button"
            role="menuitem"
            disabled={item.disabled}
            onClick={() => {
              if (item.disabled) return
              item.onClick()
              onClose()
            }}
            className={`flex w-full items-center gap-2 px-3 py-1.5 text-left font-mono text-[11px] tracking-[0.04em] transition-colors disabled:cursor-not-allowed disabled:opacity-40 ${
              item.danger
                ? 'text-muted hover:bg-stat-red/10 hover:text-stat-red'
                : 'text-text hover:bg-accent-hot/10 hover:text-accent-hot'
            }`}
          >
            <span className="flex-1">{item.label}</span>
            {item.kbd && (
              <span className="font-mono text-[9px] text-faint">{item.kbd}</span>
            )}
          </button>
        </div>
      ))}
    </motion.div>,
    document.body,
  )
}
