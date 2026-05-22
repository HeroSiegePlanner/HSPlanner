import type { CSSProperties } from 'react'

interface CornerMarksProps {
  /** Edge length of each L-mark in px. */
  size?: number
  /** Mark opacity (0-1). */
  opacity?: number
}

// Decorative L-shaped accent brackets at a panel's four corners — the shared
// modal/panel chrome motif used across every dialog and panel in the app.
// Defaults match the modal chrome (10px / 0.55); panels pass smaller values.
export function CornerMarks({ size = 10, opacity = 0.55 }: CornerMarksProps) {
  const base: CSSProperties = {
    position: 'absolute',
    width: size,
    height: size,
    border: '1px solid var(--color-accent-deep)',
    opacity,
    pointerEvents: 'none',
  }
  return (
    <>
      <span style={{ ...base, top: -1, left: -1, borderRight: 'none', borderBottom: 'none' }} />
      <span style={{ ...base, top: -1, right: -1, borderLeft: 'none', borderBottom: 'none' }} />
      <span style={{ ...base, bottom: -1, left: -1, borderRight: 'none', borderTop: 'none' }} />
      <span style={{ ...base, bottom: -1, right: -1, borderLeft: 'none', borderTop: 'none' }} />
    </>
  )
}
