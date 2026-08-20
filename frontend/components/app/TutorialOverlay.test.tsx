import { describe, expect, test, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import TutorialOverlay from './TutorialOverlay'
import {
  CARD_WIDTH,
  TUTORIAL_DONE_KEY,
  TUTORIAL_STEPS,
  placeCard,
} from './tutorialModel'

const vp = { width: 1280, height: 800 }

describe('placeCard', () => {
  test('centers card when there is no target', () => {
    expect(placeCard(null, vp)).toEqual({
      placement: 'center',
      top: 400,
      left: 640,
    })
  })

  test('places card below a target with room underneath', () => {
    const pos = placeCard({ top: 40, left: 300, width: 200, height: 30 }, vp)
    expect(pos.placement).toBe('below')
    expect(pos.top).toBeGreaterThan(70)
    expect(pos.left).toBe(300 + 100 - CARD_WIDTH / 2)
  })

  test('places card above a target near the bottom edge', () => {
    const pos = placeCard({ top: 760, left: 100, width: 200, height: 30 }, vp)
    expect(pos.placement).toBe('above')
    expect(pos.top).toBeLessThan(760)
  })

  test('uses the measured card height to pick the placement', () => {
    const target = { top: 380, left: 300, width: 200, height: 30 }
    expect(placeCard(target, vp).placement).toBe('center')
    expect(placeCard(target, vp, 300).placement).toBe('below')
  })

  test('centers card when it fits neither below nor above the target', () => {
    const small = { width: 400, height: 225 }
    const pos = placeCard({ top: 0, left: 130, width: 270, height: 44 }, small)
    expect(pos.placement).toBe('center')
  })

  test('centers card over targets taller than half the viewport', () => {
    const pos = placeCard({ top: 50, left: 300, width: 900, height: 700 }, vp)
    expect(pos.placement).toBe('center')
  })

  test('clamps card inside the horizontal viewport edges', () => {
    const nearLeft = placeCard({ top: 40, left: 0, width: 20, height: 30 }, vp)
    expect(nearLeft.left).toBeGreaterThanOrEqual(0)
    const nearRight = placeCard(
      { top: 40, left: 1260, width: 20, height: 30 },
      vp,
    )
    expect(nearRight.left + CARD_WIDTH).toBeLessThanOrEqual(vp.width)
  })
})

describe('TUTORIAL_STEPS', () => {
  test('starts and ends with a centered step', () => {
    expect(TUTORIAL_STEPS[0].target).toBeUndefined()
    expect(TUTORIAL_STEPS[TUTORIAL_STEPS.length - 1].target).toBeUndefined()
  })

  test('every step has a title and body', () => {
    for (const step of TUTORIAL_STEPS) {
      expect(step.title.length).toBeGreaterThan(0)
      expect(step.body.length).toBeGreaterThan(0)
    }
  })

  test('every view-targeting step pins the section it describes', () => {
    for (const step of TUTORIAL_STEPS) {
      if (step.target === 'view') expect(step.section).toBeDefined()
    }
  })

  test('every step that acts also knows how to undo', () => {
    for (const step of TUTORIAL_STEPS) {
      if (step.act) expect(step.undo).toBeDefined()
    }
  })
})

function renderOverlay() {
  const onClose = vi.fn()
  const setSection = vi.fn()
  render(
    <TutorialOverlay section="gear" setSection={setSection} onClose={onClose} />,
  )
  return { onClose, setSection }
}

describe('TutorialOverlay', () => {
  test('shows the first step with a progress counter', () => {
    renderOverlay()
    expect(screen.getByRole('dialog')).toBeInTheDocument()
    expect(screen.getByText(TUTORIAL_STEPS[0].title)).toBeInTheDocument()
    expect(
      screen.getByText(`Tutorial · 1 / ${TUTORIAL_STEPS.length}`),
    ).toBeInTheDocument()
  })

  test('renders the fish mascot as pure decoration', () => {
    renderOverlay()
    const img = screen.getByRole('dialog').querySelector('img')
    expect(img).not.toBeNull()
    expect(img).toHaveAttribute('alt', '')
    expect(img).toHaveAttribute('aria-hidden')
  })

  test('Next and Back move between steps', () => {
    renderOverlay()
    fireEvent.click(screen.getByRole('button', { name: 'Next' }))
    expect(screen.getByText(TUTORIAL_STEPS[1].title)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Back' }))
    expect(screen.getByText(TUTORIAL_STEPS[0].title)).toBeInTheDocument()
  })

  test('Back is disabled on the first step', () => {
    renderOverlay()
    expect(screen.getByRole('button', { name: 'Back' })).toBeDisabled()
  })

  test('arrow keys move between steps', () => {
    renderOverlay()
    fireEvent.keyDown(window, { key: 'ArrowRight' })
    expect(screen.getByText(TUTORIAL_STEPS[1].title)).toBeInTheDocument()
    fireEvent.keyDown(window, { key: 'ArrowLeft' })
    expect(screen.getByText(TUTORIAL_STEPS[0].title)).toBeInTheDocument()
  })

  test('switches to the section a step demands', () => {
    const { setSection } = renderOverlay()
    const treeIndex = TUTORIAL_STEPS.findIndex((s) => s.section === 'tree')
    expect(treeIndex).toBeGreaterThan(0)
    const next = screen.getByRole('button', { name: 'Next' })
    for (let i = 0; i < treeIndex; i++) fireEvent.click(next)
    expect(setSection).toHaveBeenCalledWith('tree')
  })

  test('Skip closes, marks the tour done and restores the section', () => {
    const { onClose, setSection } = renderOverlay()
    fireEvent.click(screen.getByRole('button', { name: 'Skip tour' }))
    expect(onClose).toHaveBeenCalledOnce()
    expect(window.localStorage.getItem(TUTORIAL_DONE_KEY)).toBe('1')
    expect(setSection).toHaveBeenLastCalledWith('gear')
  })

  test('Escape closes and marks the tour done', () => {
    const { onClose } = renderOverlay()
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(onClose).toHaveBeenCalledOnce()
    expect(window.localStorage.getItem(TUTORIAL_DONE_KEY)).toBe('1')
  })

  test('act clicks its element on entry and undo clicks on leave', async () => {
    const actSpy = vi.fn()
    const undoSpy = vi.fn()
    const actBtn = document.createElement('button')
    actBtn.setAttribute('data-tour', 'tree-suggest')
    actBtn.addEventListener('click', actSpy)
    document.body.appendChild(actBtn)
    const modal = document.createElement('div')
    modal.setAttribute('data-tour', 'suggest-modal')
    const closeBtn = document.createElement('button')
    closeBtn.setAttribute('aria-label', 'Close')
    closeBtn.addEventListener('click', undoSpy)
    modal.appendChild(closeBtn)
    document.body.appendChild(modal)

    try {
      renderOverlay()
      const suggestIndex = TUTORIAL_STEPS.findIndex(
        (s) => s.act === '[data-tour="tree-suggest"]',
      )
      expect(suggestIndex).toBeGreaterThan(0)
      const next = screen.getByRole('button', { name: 'Next' })
      for (let i = 0; i < suggestIndex; i++) fireEvent.click(next)
      await waitFor(() => expect(actSpy).toHaveBeenCalledOnce())
      expect(undoSpy).not.toHaveBeenCalled()
      fireEvent.click(next)
      await waitFor(() => expect(undoSpy).toHaveBeenCalledOnce())
    } finally {
      actBtn.remove()
      modal.remove()
    }
  })

  test('Finish on the last step closes the tour', () => {
    const { onClose } = renderOverlay()
    const next = screen.getByRole('button', { name: 'Next' })
    for (let i = 0; i < TUTORIAL_STEPS.length - 1; i++) fireEvent.click(next)
    fireEvent.click(screen.getByRole('button', { name: 'Finish' }))
    expect(onClose).toHaveBeenCalledOnce()
    expect(window.localStorage.getItem(TUTORIAL_DONE_KEY)).toBe('1')
  })
})
