import type { CSSProperties } from 'react'

interface CornerMarksProps {
  size?: number
  opacity?: number
}

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
