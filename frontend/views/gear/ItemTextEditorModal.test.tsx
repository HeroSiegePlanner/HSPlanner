import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { items } from '@data'
import ItemTextEditorModal from './ItemTextEditorModal'

// eslint-disable-next-line @typescript-eslint/consistent-type-imports
type FormatModule = typeof import('../../utils/item/itemTextFormat')

const TEXT = `Rarity: ANGELIC
Commander's Sentry Blaster
Gun
--------
Stars: 0
--------
Implicit:
+3 to All Skills
--------
Affixes:`

vi.mock('../../utils/item/itemTextFormat', async (importOriginal) => {
  const original = await importOriginal<FormatModule>()
  return {
    ...original,
    serializeEquippedItem: vi.fn(async () => TEXT),
    parseItemText: vi.fn(async () => ({ equipped: null, errors: [] })),
  }
})

describe('ItemTextEditorModal — custom affixes list', () => {
  it('inserts a [custom] implicit line for the picked stat and selects its value', async () => {
    const base = items.find(
      (it) => it.id === 'gun_angelic_commander_s_sentry_blaster',
    )!
    render(
      <ItemTextEditorModal
        slotName="Weapon"
        equipped={{
          baseId: base.id,
          affixes: [],
          socketCount: 0,
          socketed: [],
          socketTypes: [],
          stars: 0,
        }}
        base={base}
        onSave={() => {}}
        onClose={() => {}}
      />,
    )
    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement
    await waitFor(() => expect(textarea.value).toBe(TEXT))

    await userEvent.type(screen.getByRole('searchbox'), 'increased strength')
    await userEvent.click(
      screen.getByRole('button', { name: '+1% Increased Strength [custom]' }),
    )

    expect(textarea.value).toContain(
      'Implicit:\n+3 to All Skills\n+1% Increased Strength [custom]\n--------\nAffixes:',
    )
    const offset = textarea.value.indexOf('+1% Increased Strength')
    expect([textarea.selectionStart, textarea.selectionEnd]).toEqual([
      offset + 1,
      offset + 2,
    ])
  })
})
