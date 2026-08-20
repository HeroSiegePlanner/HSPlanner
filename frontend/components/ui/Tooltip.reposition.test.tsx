import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import Tooltip from './Tooltip'

type RoCallback = (entries: unknown[], observer: unknown) => void

class FakeResizeObserver {
  static last: FakeResizeObserver | null = null
  callback: RoCallback
  observed: Element[] = []
  constructor(callback: RoCallback) {
    this.callback = callback
    FakeResizeObserver.last = this
  }
  observe(el: Element) {
    this.observed.push(el)
  }
  unobserve() {}
  disconnect() {}
}

function mockRect(el: Element, rect: Partial<DOMRect>) {
  Object.defineProperty(el, 'getBoundingClientRect', {
    configurable: true,
    value: () =>
      ({
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        width: 0,
        height: 0,
        x: 0,
        y: 0,
        toJSON: () => ({}),
        ...rect,
      }) as DOMRect,
  })
}

describe('Tooltip — reclamps position when content grows', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    FakeResizeObserver.last = null
  })

  it('repositions on resize so the tooltip stays inside the viewport', async () => {
    vi.stubGlobal('ResizeObserver', FakeResizeObserver)
    // jsdom viewport: 1024x768
    render(
      <Tooltip content={<div>body</div>} delay={0}>
        <span>trig</span>
      </Tooltip>,
    )
    const trigger = screen.getByText('trig').parentElement!
    mockRect(trigger, {
      top: 600,
      bottom: 620,
      left: 50,
      right: 100,
      width: 50,
      height: 20,
    })

    await userEvent.hover(trigger)
    const tooltip = await screen.findByRole('tooltip')
    expect(FakeResizeObserver.last?.observed).toContain(tooltip)

    // content grew after the initial measure: 300x700 no longer fits below
    mockRect(tooltip, { width: 300, height: 700 })
    act(() => {
      FakeResizeObserver.last!.callback([], FakeResizeObserver.last)
    })

    await waitFor(() => {
      // clamped to bottom edge: 768 - 700 - 8 = 60
      expect(tooltip.style.top).toBe('60px')
      // placement right: trigger.right + 8 = 108
      expect(tooltip.style.left).toBe('108px')
    })
  })
})
