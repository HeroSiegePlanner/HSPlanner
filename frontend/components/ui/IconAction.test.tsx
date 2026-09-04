import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { IconAction } from './IconAction'

const icon = <svg viewBox="0 0 24 24" />

// Tooltip observes its own size to reclamp; jsdom ships no ResizeObserver.
class NoopResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', NoopResizeObserver)
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('IconAction', () => {
  it('keeps the native title when there is no styled tooltip', () => {
    render(
      <IconAction label="Rename" onClick={() => {}}>
        {icon}
      </IconAction>,
    )
    expect(screen.getByRole('button', { name: 'Rename' })).toHaveAttribute(
      'title',
      'Rename',
    )
  })

  it('drops the native title once the styled tooltip takes over', () => {
    render(
      <IconAction label="Rename" tooltipPlacement="left" onClick={() => {}}>
        {icon}
      </IconAction>,
    )
    expect(screen.getByRole('button', { name: 'Rename' })).not.toHaveAttribute('title')
  })

  it('lets a disabled button hand its pointer events to the tooltip wrapper', async () => {
    // A disabled <button> emits no pointer events, so without this the label
    // explaining *why* it is disabled could never open.
    render(
      <IconAction
        label="All 8 stages used"
        tooltipPlacement="left"
        disabled
        onClick={() => {}}
      >
        {icon}
      </IconAction>,
    )
    const button = screen.getByRole('button', { name: 'All 8 stages used' })
    expect(button).toBeDisabled()
    expect(button.className).toContain('disabled:pointer-events-none')

    fireEvent.mouseEnter(button.parentElement!)
    await waitFor(() =>
      expect(screen.getByRole('tooltip')).toHaveTextContent('All 8 stages used'),
    )
  })

  it('does not surrender pointer events when it has no tooltip to show', () => {
    render(
      <IconAction label="Delete" disabled onClick={() => {}}>
        {icon}
      </IconAction>,
    )
    expect(screen.getByRole('button', { name: 'Delete' }).className).not.toContain(
      'pointer-events-none',
    )
  })
})
