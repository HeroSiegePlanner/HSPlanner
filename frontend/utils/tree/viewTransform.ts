export interface ViewTransform {
  scale: number
  tx: number
  ty: number
}

const ZOOM_IN_FACTOR = 1.1
const ZOOM_OUT_FACTOR = 0.9

// Zoom toward the cursor: the world point under (mx, my) stays fixed on screen.
export function zoomAtPoint(
  view: ViewTransform,
  mx: number,
  my: number,
  zoomIn: boolean,
  minScale: number,
  maxScale: number,
): ViewTransform {
  const factor = zoomIn ? ZOOM_IN_FACTOR : ZOOM_OUT_FACTOR
  const scale = Math.max(minScale, Math.min(maxScale, view.scale * factor))
  const worldX = (mx - view.tx) / view.scale
  const worldY = (my - view.ty) / view.scale
  return { scale, tx: mx - worldX * scale, ty: my - worldY * scale }
}
