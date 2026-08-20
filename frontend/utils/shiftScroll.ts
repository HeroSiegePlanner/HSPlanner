// Browsers turn shift+wheel into a horizontal scroll (deltaX), so panels that
// only scroll vertically stop reacting while shift is held. Route the delta
// back into the nearest vertical-only scroll container.

export function findVerticalScrollTarget(
  start: EventTarget | null,
): HTMLElement | null {
  let el = start instanceof Element ? start : null
  while (el) {
    if (el instanceof HTMLElement) {
      const style = getComputedStyle(el)
      const scrollableX =
        el.scrollWidth > el.clientWidth && /auto|scroll/.test(style.overflowX)
      // horizontally scrollable container — native shift+wheel is the right behavior
      if (scrollableX) return null
      const scrollableY =
        el.scrollHeight > el.clientHeight && /auto|scroll/.test(style.overflowY)
      if (scrollableY) return el
    }
    el = el.parentElement
  }
  return null
}

export function initShiftScroll(): () => void {
  const handler = (e: WheelEvent) => {
    if (!e.shiftKey || e.defaultPrevented) return
    // with shift held most engines report the delta on deltaX
    const delta = e.deltaX !== 0 ? e.deltaX : e.deltaY
    if (delta === 0) return
    const target = findVerticalScrollTarget(e.target)
    if (!target) return
    e.preventDefault()
    target.scrollTop += delta
  }
  window.addEventListener('wheel', handler, { passive: false })
  return () => window.removeEventListener('wheel', handler)
}
