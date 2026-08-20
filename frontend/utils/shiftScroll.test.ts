import { afterEach, describe, expect, it } from 'vitest'
import { findVerticalScrollTarget, initShiftScroll } from './shiftScroll'

function makeEl(opts: {
  overflowY?: string
  overflowX?: string
  scrollH?: number
  clientH?: number
  scrollW?: number
  clientW?: number
  parent?: HTMLElement
}): HTMLElement {
  const el = document.createElement('div')
  el.style.overflowY = opts.overflowY ?? 'visible'
  el.style.overflowX = opts.overflowX ?? 'visible'
  Object.defineProperty(el, 'scrollHeight', { value: opts.scrollH ?? 0 })
  Object.defineProperty(el, 'clientHeight', { value: opts.clientH ?? 0 })
  Object.defineProperty(el, 'scrollWidth', { value: opts.scrollW ?? 0 })
  Object.defineProperty(el, 'clientWidth', { value: opts.clientW ?? 0 })
  ;(opts.parent ?? document.body).appendChild(el)
  return el
}

function wheel(opts: Partial<WheelEventInit>): WheelEvent {
  return new WheelEvent('wheel', { bubbles: true, cancelable: true, ...opts })
}

afterEach(() => {
  document.body.innerHTML = ''
})

describe('findVerticalScrollTarget', () => {
  it('finds the nearest vertically scrollable ancestor', () => {
    const scroller = makeEl({ overflowY: 'auto', scrollH: 500, clientH: 100 })
    const child = makeEl({ parent: scroller })
    expect(findVerticalScrollTarget(child)).toBe(scroller)
  })

  it('returns null when a horizontally scrollable container is closer', () => {
    const outer = makeEl({ overflowY: 'auto', scrollH: 500, clientH: 100 })
    const horizontal = makeEl({
      overflowX: 'auto',
      scrollW: 500,
      clientW: 100,
      parent: outer,
    })
    const child = makeEl({ parent: horizontal })
    expect(findVerticalScrollTarget(child)).toBeNull()
  })

  it('returns null without any scrollable ancestor', () => {
    const child = makeEl({})
    expect(findVerticalScrollTarget(child)).toBeNull()
  })
})

describe('initShiftScroll', () => {
  it('scrolls the vertical container on shift+wheel (axis-swapped delta)', () => {
    const dispose = initShiftScroll()
    const scroller = makeEl({ overflowY: 'auto', scrollH: 500, clientH: 100 })
    const child = makeEl({ parent: scroller })
    scroller.scrollTop = 0

    const e = wheel({ shiftKey: true, deltaX: 40, deltaY: 0 })
    child.dispatchEvent(e)

    expect(scroller.scrollTop).toBe(40)
    expect(e.defaultPrevented).toBe(true)
    dispose()
  })

  it('falls back to deltaY when the engine does not swap axes', () => {
    const dispose = initShiftScroll()
    const scroller = makeEl({ overflowY: 'auto', scrollH: 500, clientH: 100 })
    scroller.scrollTop = 10

    scroller.dispatchEvent(wheel({ shiftKey: true, deltaX: 0, deltaY: -10 }))

    expect(scroller.scrollTop).toBe(0)
    dispose()
  })

  it('ignores wheel without shift and already-handled events', () => {
    const dispose = initShiftScroll()
    const scroller = makeEl({ overflowY: 'auto', scrollH: 500, clientH: 100 })
    scroller.scrollTop = 0

    scroller.dispatchEvent(wheel({ shiftKey: false, deltaY: 40 }))
    expect(scroller.scrollTop).toBe(0)

    const handled = wheel({ shiftKey: true, deltaX: 40 })
    handled.preventDefault()
    scroller.dispatchEvent(handled)
    expect(scroller.scrollTop).toBe(0)
    dispose()
  })

  it('stops listening after dispose', () => {
    const dispose = initShiftScroll()
    dispose()
    const scroller = makeEl({ overflowY: 'auto', scrollH: 500, clientH: 100 })
    scroller.scrollTop = 0
    scroller.dispatchEvent(wheel({ shiftKey: true, deltaX: 40 }))
    expect(scroller.scrollTop).toBe(0)
  })
})
