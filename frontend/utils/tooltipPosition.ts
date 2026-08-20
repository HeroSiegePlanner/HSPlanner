export interface Size {
  width: number
  height: number
}

export interface Point {
  x: number
  y: number
}

export function clampTooltipToViewport(
  anchor: Point,
  size: Size,
  viewport: Size,
  offset = 18,
  margin = 12,
): { left: number; top: number } {
  let left = anchor.x + offset
  if (left + size.width + margin > viewport.width) {
    left = anchor.x - size.width - offset
  }
  left = Math.max(margin, Math.min(left, viewport.width - size.width - margin))

  let top = anchor.y + offset
  if (top + size.height + margin > viewport.height) {
    top = anchor.y - size.height - offset
  }
  top = Math.max(margin, Math.min(top, viewport.height - size.height - margin))

  return { left, top }
}
