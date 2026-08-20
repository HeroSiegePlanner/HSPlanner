import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import Dropdown, { type DropdownOption } from './Dropdown'

const OPTIONS: DropdownOption[] = [
  { id: 'a', label: 'Amazon' },
  { id: 'b', label: 'Bard' },
  { id: 'c', label: 'Butcher' },
]

describe('<Dropdown>', () => {
  it('shows the placeholder when nothing is selected', () => {
    render(
      <Dropdown
        value={null}
        options={OPTIONS}
        onChange={() => {}}
        placeholder="Pick one"
      />,
    )
    expect(screen.getByRole('button')).toHaveTextContent('Pick one')
  })

  it('shows the selected option label', () => {
    render(<Dropdown value="b" options={OPTIONS} onChange={() => {}} />)
    expect(screen.getByRole('button')).toHaveTextContent('Bard')
  })

  it('opens the listbox on trigger click and lists every option', async () => {
    render(<Dropdown value={null} options={OPTIONS} onChange={() => {}} />)
    await userEvent.click(screen.getByRole('button'))
    expect(screen.getByRole('listbox')).toBeInTheDocument()
    expect(screen.getAllByRole('option')).toHaveLength(3)
  })

  it('calls onChange with the id and closes when an option is picked', async () => {
    const onChange = vi.fn()
    render(<Dropdown value={null} options={OPTIONS} onChange={onChange} />)
    await userEvent.click(screen.getByRole('button'))
    await userEvent.click(screen.getByText('Butcher'))
    expect(onChange).toHaveBeenCalledWith('c')
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
  })

  it('renders a search box and footer by default (searchable)', async () => {
    render(
      <Dropdown
        value={null}
        options={OPTIONS}
        onChange={() => {}}
        searchPlaceholder="Find…"
      />,
    )
    await userEvent.click(screen.getByRole('button'))
    expect(screen.getByPlaceholderText('Find…')).toBeInTheDocument()
    expect(document.querySelector('.hs-dd-foot')).toBeInTheDocument()
  })

  it('filters options by the search query', async () => {
    render(<Dropdown value={null} options={OPTIONS} onChange={() => {}} />)
    await userEvent.click(screen.getByRole('button'))
    await userEvent.type(screen.getByRole('textbox'), 'bar')
    expect(screen.getAllByRole('option')).toHaveLength(1)
    expect(screen.getByText('Bard')).toBeInTheDocument()
  })

  it('hides the search box and footer when searchable is false', async () => {
    render(
      <Dropdown
        value={null}
        options={OPTIONS}
        onChange={() => {}}
        searchable={false}
      />,
    )
    await userEvent.click(screen.getByRole('button'))
    expect(screen.getByRole('listbox')).toBeInTheDocument()
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
    expect(document.querySelector('.hs-dd-foot')).not.toBeInTheDocument()
  })

  it('supports arrow-key navigation and Enter selection when not searchable', async () => {
    const onChange = vi.fn()
    render(
      <Dropdown
        value={null}
        options={OPTIONS}
        onChange={onChange}
        searchable={false}
      />,
    )
    await userEvent.click(screen.getByRole('button'))
    const listbox = screen.getByRole('listbox')
    fireEvent.keyDown(listbox, { key: 'ArrowDown' })
    fireEvent.keyDown(listbox, { key: 'Enter' })
    expect(onChange).toHaveBeenCalledWith('b')
  })

  it('closes on Escape when not searchable', async () => {
    render(
      <Dropdown
        value={null}
        options={OPTIONS}
        onChange={() => {}}
        searchable={false}
      />,
    )
    await userEvent.click(screen.getByRole('button'))
    const listbox = screen.getByRole('listbox')
    fireEvent.keyDown(listbox, { key: 'Escape' })
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
  })

  it('applies the compact modifier class', () => {
    const { container } = render(
      <Dropdown value={null} options={OPTIONS} onChange={() => {}} compact />,
    )
    expect(container.querySelector('.hs-dd')).toHaveClass('hs-dd--compact')
  })

  it('portals the menu to document.body so overflow-hidden parents cannot clip it', async () => {
    render(
      <div style={{ overflow: 'hidden' }}>
        <Dropdown
          value={null}
          options={OPTIONS}
          onChange={() => {}}
          searchable={false}
        />
      </div>,
    )
    await userEvent.click(screen.getByRole('button'))
    const menu = document.querySelector('.hs-dd-menu')
    expect(menu).toBeInTheDocument()
    expect(menu?.parentElement).toBe(document.body)
  })
})
