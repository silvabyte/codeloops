import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react'
import { CopyButton } from '../CopyButton'

describe('CopyButton', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('should render clipboard icon by default', () => {
    render(<CopyButton content="test content" />)

    const button = screen.getByRole('button', { name: /copy to clipboard/i })
    expect(button).toBeInTheDocument()
  })

  it('should copy content to clipboard when clicked', async () => {
    render(<CopyButton content="test content" />)

    const button = screen.getByRole('button')
    fireEvent.click(button)

    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith('test content')
    })
  })

  it('should show checkmark icon after successful copy', async () => {
    render(<CopyButton content="test content" />)

    const button = screen.getByRole('button')
    fireEvent.click(button)

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /copied/i })).toBeInTheDocument()
    })
  })

  it('should revert to clipboard icon after 2 seconds', async () => {
    vi.useFakeTimers()

    render(<CopyButton content="test content" />)

    const button = screen.getByRole('button')

    // Click and flush only the microtask queue (for the clipboard promise)
    await act(async () => {
      fireEvent.click(button)
      // Flush microtasks but not the 2000ms setTimeout
      await Promise.resolve()
    })

    // After clicking, it should show copied
    expect(screen.getByRole('button', { name: /copied/i })).toBeInTheDocument()

    // Now advance past the 2 second timeout
    await act(async () => {
      vi.advanceTimersByTime(2100)
    })

    expect(screen.getByRole('button', { name: /copy to clipboard/i })).toBeInTheDocument()
  })

  it('should accept additional className', () => {
    render(<CopyButton content="test" className="extra-class" />)

    const button = screen.getByRole('button')
    expect(button).toHaveClass('extra-class')
  })
})
