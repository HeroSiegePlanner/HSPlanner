import type { CSSProperties, ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { motion } from 'motion/react'
import { backdropVariants, panelVariants } from '../lib/motion'

export const MODAL_FOOTER_CLASS =
  'flex items-center justify-end gap-2 border-t border-border px-6 py-3'

export const MODAL_BTN_CLASS =
  'rounded-[3px] border border-border-2 bg-transparent px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-40'

export const MODAL_BTN_PRIMARY_CLASS =
  'rounded-[3px] border border-accent-deep px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4] disabled:cursor-not-allowed disabled:opacity-60'

interface ModalProps {
  onClose: () => void
  panelClassName: string
  eyebrow: ReactNode
  title: ReactNode
  titleId?: string
  titleClassName?: string
  subtitle?: ReactNode
  closeDisabled?: boolean
  headerActions?: ReactNode
  backdropClassName?: string
  panelStyle?: CSSProperties
  portal?: boolean
  children: ReactNode
}

export function Modal({
  onClose,
  panelClassName,
  eyebrow,
  title,
  titleId,
  titleClassName,
  subtitle,
  closeDisabled,
  headerActions,
  backdropClassName,
  panelStyle,
  portal = true,
  children,
}: ModalProps) {
  const tree = (
    <motion.div
      role="presentation"
      className={`fixed inset-0 z-100 flex items-center justify-center backdrop-blur-sm${backdropClassName ? ` ${backdropClassName}` : ''}`}
      onMouseDown={onClose}
      variants={backdropVariants}
      initial="initial"
      animate="animate"
      style={{
        background:
          'radial-gradient(ellipse at 50% 0%, rgba(201,165,90,0.06), rgba(0,0,0,0.78) 60%)',
      }}
    >
      <motion.div
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onMouseDown={(e) => e.stopPropagation()}
        variants={panelVariants}
        initial="initial"
        animate="animate"
        className={`relative flex flex-col overflow-hidden rounded-xl border border-border ${panelClassName}`}
        style={{
          background:
            'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 86%, transparent))',
          boxShadow: '0 20px 60px rgba(0,0,0,0.55)',
          ...panelStyle,
        }}
      >
        <header className="flex items-start justify-between gap-3 border-b border-border px-6 py-4">
          <div className="min-w-0">
            <div className="mb-1.5 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.12em] text-faint">
              <span
                aria-hidden
                className="inline-block h-1 w-1 rounded-full bg-accent"
              />
              {eyebrow}
            </div>
            <h2
              id={titleId}
              className={`m-0 text-[17px] font-semibold tracking-[-0.01em] text-text ${titleClassName ?? ''}`}
            >
              {title}
            </h2>
            {subtitle != null && (
              <div className="mt-1 text-[12px] text-muted">{subtitle}</div>
            )}
          </div>
          <div className="flex shrink-0 items-center gap-2">
            {headerActions}
            <button
              type="button"
              onClick={onClose}
              disabled={closeDisabled}
              aria-label="Close"
              className="rounded-md border border-border px-3 py-1.5 text-[12px] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-40"
            >
              Close
            </button>
          </div>
        </header>

        {children}
      </motion.div>
    </motion.div>
  )
  return portal ? createPortal(tree, document.body) : tree
}
