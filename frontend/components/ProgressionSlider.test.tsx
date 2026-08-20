import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import ProgressionSlider from './ProgressionSlider'

describe('<ProgressionSlider>', () => {
  it('renders nothing when there are fewer than 2 steps', () => {
    const { container } = render(
      <ProgressionSlider total={1} value={null} onChange={() => {}} />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('shows the current step and total', () => {
    render(<ProgressionSlider total={30} value={7} onChange={() => {}} />)
    expect(screen.getByText('7 / 30')).toBeInTheDocument()
    expect(screen.getByRole('slider')).toHaveValue('7')
  })

  it('treats null value as live mode at the last step', () => {
    render(<ProgressionSlider total={30} value={null} onChange={() => {}} />)
    expect(screen.getByText('30 / 30')).toBeInTheDocument()
    expect(screen.getByRole('slider')).toHaveValue('30')
  })

  it('reports an intermediate step as a number', () => {
    const onChange = vi.fn()
    render(<ProgressionSlider total={30} value={null} onChange={onChange} />)
    fireEvent.change(screen.getByRole('slider'), { target: { value: '12' } })
    expect(onChange).toHaveBeenCalledWith(12)
  })

  it('reports the last step as null (back to live mode)', () => {
    const onChange = vi.fn()
    render(<ProgressionSlider total={30} value={12} onChange={onChange} />)
    fireEvent.change(screen.getByRole('slider'), { target: { value: '30' } })
    expect(onChange).toHaveBeenCalledWith(null)
  })
})
