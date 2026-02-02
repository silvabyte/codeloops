import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { ContentBlock } from '../ContentBlock'

describe('ContentBlock', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('rendering', () => {
    it('should render label', () => {
      render(<ContentBlock label="PROMPT" content="Test content" />)

      expect(screen.getByText('PROMPT')).toBeInTheDocument()
    })

    it('should render content', () => {
      render(<ContentBlock label="ACTOR" content="Actor output here" />)

      expect(screen.getByText('Actor output here')).toBeInTheDocument()
    })

    it('should render copy button', () => {
      render(<ContentBlock label="TEST" content="Content to copy" />)

      expect(screen.getByRole('button', { name: /copy to clipboard/i })).toBeInTheDocument()
    })
  })

  describe('variants', () => {
    it('should apply actor variant styling', () => {
      const { container } = render(
        <ContentBlock label="ACTOR" content="Actor output" variant="actor" />
      )

      const block = container.firstChild as HTMLElement
      expect(block).toHaveClass('border-l-cyan-dim')
    })

    it('should apply critic variant styling', () => {
      const { container } = render(
        <ContentBlock label="CRITIC" content="Critic feedback" variant="critic" />
      )

      const block = container.firstChild as HTMLElement
      expect(block).toHaveClass('border-l-amber-dim')
    })

    it('should not apply left border for default variant', () => {
      const { container } = render(
        <ContentBlock label="DEFAULT" content="Default content" variant="default" />
      )

      const block = container.firstChild as HTMLElement
      expect(block).not.toHaveClass('border-l-cyan-dim')
      expect(block).not.toHaveClass('border-l-amber-dim')
    })

    it('should default to no left border when variant not specified', () => {
      const { container } = render(
        <ContentBlock label="NO VARIANT" content="Content" />
      )

      const block = container.firstChild as HTMLElement
      expect(block).not.toHaveClass('border-l-cyan-dim')
      expect(block).not.toHaveClass('border-l-amber-dim')
    })
  })

  describe('copy functionality', () => {
    it('should copy content when copy button is clicked', async () => {
      render(<ContentBlock label="TEST" content="Copy this text" />)

      const copyButton = screen.getByRole('button', { name: /copy to clipboard/i })
      fireEvent.click(copyButton)

      await waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalledWith('Copy this text')
      })
    })
  })

  describe('styling', () => {
    it('should accept additional className', () => {
      const { container } = render(
        <ContentBlock label="TEST" content="Content" className="custom-class" />
      )

      const block = container.firstChild as HTMLElement
      expect(block).toHaveClass('custom-class')
    })

    it('should preserve whitespace in content', () => {
      render(<ContentBlock label="TEST" content="Line 1\n  Line 2\n    Line 3" />)

      const pre = screen.getByText(/Line 1/)
      expect(pre).toHaveClass('whitespace-pre-wrap')
    })
  })
})
