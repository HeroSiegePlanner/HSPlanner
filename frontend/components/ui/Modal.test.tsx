import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { Modal } from './Modal'

function renderModal(props: Partial<Parameters<typeof Modal>[0]> = {}) {
  return render(
    <Modal onClose={() => {}} panelClassName="" eyebrow="Test" title="Gear Slot" {...props}>
      <button type="button">Inside</button>
    </Modal>,
  )
}

describe('<Modal>', () => {
  it('names the dialog with its own title', () => {
    renderModal()
    expect(screen.getByRole('dialog', { name: 'Gear Slot' })).toBeInTheDocument()
  })

  it('moves focus into the dialog on open', () => {
    renderModal()
    expect(screen.getByRole('dialog')).toHaveFocus()
  })

  it('closes on Escape', async () => {
    const onClose = vi.fn()
    renderModal({ onClose })
    await userEvent.keyboard('{Escape}')
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('prefers onEscape over onClose when given', async () => {
    const onClose = vi.fn()
    const onEscape = vi.fn()
    renderModal({ onClose, onEscape })
    await userEvent.keyboard('{Escape}')
    expect(onEscape).toHaveBeenCalledTimes(1)
    expect(onClose).not.toHaveBeenCalled()
  })

  it('ignores Escape while closing is disabled', async () => {
    const onClose = vi.fn()
    renderModal({ onClose, closeDisabled: true })
    await userEvent.keyboard('{Escape}')
    expect(onClose).not.toHaveBeenCalled()
  })

  it('only the topmost dialog reacts to Escape', async () => {
    const outer = vi.fn()
    const inner = vi.fn()
    function Nested({ withInner }: { withInner: boolean }) {
      return (
        <Modal onClose={outer} panelClassName="" eyebrow="Outer" title="Outer">
          {withInner && (
            <Modal onClose={inner} panelClassName="" eyebrow="Inner" title="Inner">
              <span>nested</span>
            </Modal>
          )}
        </Modal>
      )
    }

    const { rerender } = render(<Nested withInner={false} />)
    rerender(<Nested withInner />)
    await userEvent.keyboard('{Escape}')
    expect(inner).toHaveBeenCalledTimes(1)
    expect(outer).not.toHaveBeenCalled()

    rerender(<Nested withInner={false} />)
    await userEvent.keyboard('{Escape}')
    expect(outer).toHaveBeenCalledTimes(1)
  })

  it('restores focus to the trigger and un-inerts the app root on close', () => {
    document.body.innerHTML = '<div id="root"></div>'
    const trigger = document.createElement('button')
    document.getElementById('root')!.append(trigger)
    trigger.focus()

    const { unmount } = renderModal()
    expect(document.getElementById('root')).toHaveAttribute('inert')

    unmount()
    expect(document.getElementById('root')).not.toHaveAttribute('inert')
    expect(trigger).toHaveFocus()
  })

  it('restores focus to the trigger even when a child autofocuses', () => {
    document.body.innerHTML = '<div id="root"></div>'
    const trigger = document.createElement('button')
    document.getElementById('root')!.append(trigger)
    trigger.focus()

    const { unmount } = render(
      <Modal onClose={() => {}} panelClassName="" eyebrow="Test" title="Picker">
        <input autoFocus aria-label="Search" />
      </Modal>,
    )
    expect(screen.getByLabelText('Search')).toHaveFocus()

    unmount()
    expect(trigger).toHaveFocus()
  })
})
