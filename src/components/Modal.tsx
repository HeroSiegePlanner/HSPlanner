import type { CSSProperties, ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { motion } from 'motion/react'
import { backdropVariants, panelVariants } from '../lib/motion'
import { CornerMarks } from './CornerMarks'

interface ModalProps {
  /** Called on backdrop click and the header Close button. */
  onClose: () => void
  /** Tailwind sizing classes for the panel, e.g. 'h-[88vh] w-[640px] max-w-[94vw]'. */
  panelClassName: string
  /** Header eyebrow line, rendered after the accent diamond. */
  eyebrow: ReactNode
  /** Header title. */
  title: ReactNode
  /** id for the <h2>, paired with aria-labelledby on the panel. */
  titleId?: string
  /** Extra classes for the <h2> (e.g. 'truncate'). */
  titleClassName?: string
  /** Optional line below the title. */
  subtitle?: ReactNode
  /** Disables the Close button (e.g. while a blocking action runs). */
  closeDisabled?: boolean
  /** Optional controls rendered in the header, left of the Close button. */
  headerActions?: ReactNode
  /** Extra classes appended to the backdrop wrapper (e.g. 'p-6'). */
  backdropClassName?: string
  /** Extra inline styles merged into the panel (e.g. a dynamic width). */
  panelStyle?: CSSProperties
  /** Render into document.body via a portal. Defaults to true. */
  portal?: boolean
  /** Panel body, rendered below the header. */
  children: ReactNode
}

// Shared chrome for the app's dialogs: a fading backdrop, a scaling panel with
// the project's CornerMarks, and the standard header (accent eyebrow, title,
// optional subtitle, Close button). Body content goes in `children`.
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
        className={`relative flex flex-col overflow-hidden rounded-[6px] border border-border ${panelClassName}`}
        style={{
          background:
            'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 80%, transparent))',
          boxShadow:
            'inset 0 1px 0 rgba(255,255,255,0.02), 0 24px 64px rgba(0,0,0,0.7)',
          ...panelStyle,
        }}
      >
        <CornerMarks />

        <header
          className="flex items-start justify-between gap-3 border-b border-border px-5 py-4"
          style={{
            background:
              'linear-gradient(180deg, rgba(201,165,90,0.05), transparent)',
          }}
        >
          <div className="min-w-0">
            <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
              <span
                aria-hidden
                className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
                style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
              />
              {eyebrow}
            </div>
            <h2
              id={titleId}
              className={`m-0 text-[18px] font-semibold tracking-[0.02em] text-accent-hot ${titleClassName ?? ''}`}
              style={{ textShadow: '0 0 16px rgba(224,184,100,0.15)' }}
            >
              {title}
            </h2>
            {subtitle != null && (
              <div className="mt-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted">
                {subtitle}
              </div>
            )}
          </div>
          <div className="flex shrink-0 items-center gap-2">
            {headerActions}
            <button
              type="button"
              onClick={onClose}
              disabled={closeDisabled}
              aria-label="Close"
              className="rounded-[3px] border border-border-2 bg-panel-2 px-3.5 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-40"
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
