import { afterEach, describe, expect, it } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import { useBuild } from '../../store/build'
import StorageErrorBanner from './StorageErrorBanner'

afterEach(() => {
  useBuild.setState({ storageError: null })
})

describe('StorageErrorBanner', () => {
  it('renders nothing when there is no storage error', () => {
    useBuild.setState({ storageError: null })

    const { container } = render(<StorageErrorBanner />)

    expect(container).toBeEmptyDOMElement()
  })

  it('shows the recorded storage error message', () => {
    useBuild.setState({ storageError: 'Could not save — disk is full.' })

    render(<StorageErrorBanner />)

    expect(
      screen.getByText('Could not save — disk is full.'),
    ).toBeInTheDocument()
  })

  it('clears the error when the dismiss button is clicked', () => {
    useBuild.setState({ storageError: 'Could not save — disk is full.' })
    render(<StorageErrorBanner />)

    fireEvent.click(screen.getByRole('button', { name: /dismiss/i }))

    expect(useBuild.getState().storageError).toBeNull()
  })
})
