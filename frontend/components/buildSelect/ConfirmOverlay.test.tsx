import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, render, screen } from '@testing-library/react'
import { ConfirmOverlay, HOLD_TO_CONFIRM_MS } from './overlays'

function renderConfirm(onConfirm: () => void) {
  return render(
    <ConfirmOverlay
      section="Delete"
      title="Delete build"
      message="Sure?"
      confirmLabel="Delete build"
      danger
      onConfirm={onConfirm}
      onClose={() => {}}
    />,
  )
}

describe('ConfirmOverlay — hold to delete', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('fires onConfirm after the danger button is held for the full duration', () => {
    const onConfirm = vi.fn()
    renderConfirm(onConfirm)
    const btn = screen.getByRole('button', { name: /delete build/i })

    fireEvent.pointerDown(btn)
    expect(onConfirm).not.toHaveBeenCalled()

    act(() => {
      vi.advanceTimersByTime(HOLD_TO_CONFIRM_MS)
    })
    expect(onConfirm).toHaveBeenCalledTimes(1)
  })

  it('does not fire on a quick click released before the hold completes', () => {
    const onConfirm = vi.fn()
    renderConfirm(onConfirm)
    const btn = screen.getByRole('button', { name: /delete build/i })

    fireEvent.pointerDown(btn)
    act(() => {
      vi.advanceTimersByTime(HOLD_TO_CONFIRM_MS / 2)
    })
    fireEvent.pointerUp(btn)
    act(() => {
      vi.advanceTimersByTime(HOLD_TO_CONFIRM_MS * 2)
    })
    fireEvent.click(btn, { detail: 1 })

    expect(onConfirm).not.toHaveBeenCalled()
  })

  it('confirms immediately on keyboard activation (a11y)', () => {
    const onConfirm = vi.fn()
    renderConfirm(onConfirm)
    const btn = screen.getByRole('button', { name: /delete build/i })

    fireEvent.click(btn, { detail: 0 })

    expect(onConfirm).toHaveBeenCalledTimes(1)
  })
})
