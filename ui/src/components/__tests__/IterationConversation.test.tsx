import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { IterationConversation } from '../IterationConversation'
import type { Iteration } from '@/api/types'

const mockIterations: Iteration[] = [
  {
    iterationNumber: 0,
    actorOutput: 'First iteration output',
    actorStderr: '',
    actorExitCode: 0,
    actorDurationSecs: 120,
    gitDiff: 'diff --git a/file.ts b/file.ts\n+new line',
    gitFilesChanged: 2,
    criticDecision: 'CONTINUE',
    feedback: 'Keep working on the implementation',
    timestamp: '2026-02-02T10:00:00Z',
  },
  {
    iterationNumber: 1,
    actorOutput: 'Second iteration completed',
    actorStderr: '',
    actorExitCode: 0,
    actorDurationSecs: 180,
    gitDiff: 'diff --git a/another.ts b/another.ts\n+another line',
    gitFilesChanged: 3,
    criticDecision: 'DONE',
    feedback: 'Task completed successfully',
    timestamp: '2026-02-02T10:03:00Z',
  },
]

describe('IterationConversation', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('empty state', () => {
    it('should show empty message when no iterations', () => {
      render(<IterationConversation iterations={[]} />)

      expect(screen.getByText('No iterations yet.')).toBeInTheDocument()
    })
  })

  describe('iteration cards', () => {
    it('should render correct number of iteration cards', () => {
      render(<IterationConversation iterations={mockIterations} />)

      expect(screen.getByText('ITERATION 0')).toBeInTheDocument()
      expect(screen.getByText('ITERATION 1')).toBeInTheDocument()
    })

    it('should display iteration duration', () => {
      render(<IterationConversation iterations={mockIterations} />)

      expect(screen.getByText('2m 0s')).toBeInTheDocument()
      expect(screen.getByText('3m 0s')).toBeInTheDocument()
    })

    it('should mark final iteration with star', () => {
      render(<IterationConversation iterations={mockIterations} />)

      // Last iteration (DONE) should have the star
      const starElements = screen.getAllByTitle('Final iteration')
      expect(starElements).toHaveLength(1)
    })
  })

  describe('actor output', () => {
    it('should display actor output', () => {
      render(<IterationConversation iterations={mockIterations} />)

      expect(screen.getByText('First iteration output')).toBeInTheDocument()
      expect(screen.getByText('Second iteration completed')).toBeInTheDocument()
    })

    it('should show placeholder when actor has no output', () => {
      const iterationsWithEmptyOutput: Iteration[] = [
        {
          ...mockIterations[0],
          actorOutput: '',
        },
      ]

      render(<IterationConversation iterations={iterationsWithEmptyOutput} />)

      expect(screen.getByText('(no output)')).toBeInTheDocument()
    })
  })

  describe('critic feedback', () => {
    it('should display critic feedback', () => {
      render(<IterationConversation iterations={mockIterations} />)

      expect(screen.getByText('Keep working on the implementation')).toBeInTheDocument()
      expect(screen.getByText('Task completed successfully')).toBeInTheDocument()
    })

    it('should show placeholder when no feedback', () => {
      const iterationsWithoutFeedback: Iteration[] = [
        {
          ...mockIterations[0],
          feedback: null,
        },
      ]

      render(<IterationConversation iterations={iterationsWithoutFeedback} />)

      expect(screen.getByText('No feedback provided')).toBeInTheDocument()
    })
  })

  describe('decision badges', () => {
    it('should display CONTINUE badge with amber styling', () => {
      render(<IterationConversation iterations={mockIterations} />)

      const continueBadge = screen.getByText('CONTINUE')
      expect(continueBadge).toHaveClass('text-amber')
      expect(continueBadge).toHaveClass('bg-amber/10')
    })

    it('should display DONE badge with success styling', () => {
      render(<IterationConversation iterations={mockIterations} />)

      const doneBadge = screen.getByText('DONE')
      expect(doneBadge).toHaveClass('text-success')
      expect(doneBadge).toHaveClass('bg-success/10')
    })

    it('should display REJECT badge with destructive styling', () => {
      const rejectIterations: Iteration[] = [
        {
          ...mockIterations[0],
          criticDecision: 'REJECT',
        },
      ]

      render(<IterationConversation iterations={rejectIterations} />)

      const rejectBadge = screen.getByText('REJECT')
      expect(rejectBadge).toHaveClass('text-destructive')
      expect(rejectBadge).toHaveClass('bg-destructive/10')
    })
  })

  describe('diff expansion', () => {
    it('should have collapsible diff section', () => {
      render(<IterationConversation iterations={mockIterations} />)

      const diffButtons = screen.getAllByText(/View diff/)
      expect(diffButtons.length).toBeGreaterThan(0)
    })

    it('should show files changed count', () => {
      render(<IterationConversation iterations={mockIterations} />)

      expect(screen.getByText('View diff (2 files changed)')).toBeInTheDocument()
      expect(screen.getByText('View diff (3 files changed)')).toBeInTheDocument()
    })

    it('should expand diff when clicked', () => {
      render(<IterationConversation iterations={mockIterations} />)

      const expandButton = screen.getByText('View diff (2 files changed)')
      fireEvent.click(expandButton)

      expect(screen.getByText('+new line')).toBeInTheDocument()
    })

    it('should collapse diff when clicked again', () => {
      render(<IterationConversation iterations={mockIterations} />)

      const expandButton = screen.getByText('View diff (2 files changed)')
      fireEvent.click(expandButton)

      expect(screen.getByText('+new line')).toBeInTheDocument()

      // Find the collapse button (arrow changed)
      const collapseButton = screen.getByText('View diff (2 files changed)')
      fireEvent.click(collapseButton)

      expect(screen.queryByText('+new line')).not.toBeInTheDocument()
    })

    it('should not show diff section when no diff', () => {
      const iterationsWithoutDiff: Iteration[] = [
        {
          ...mockIterations[0],
          gitDiff: '',
          gitFilesChanged: 0,
        },
      ]

      render(<IterationConversation iterations={iterationsWithoutDiff} />)

      expect(screen.queryByText(/View diff/)).not.toBeInTheDocument()
    })
  })

  describe('copy functionality', () => {
    it('should have copy buttons for actor output', () => {
      render(<IterationConversation iterations={mockIterations} />)

      // Each iteration should have actor copy button
      const copyButtons = screen.getAllByRole('button', { name: /copy to clipboard/i })
      expect(copyButtons.length).toBeGreaterThan(0)
    })

    it('should copy actor output when copy button clicked', async () => {
      render(<IterationConversation iterations={mockIterations} />)

      const copyButtons = screen.getAllByRole('button', { name: /copy to clipboard/i })
      // Click the first copy button (should be for actor in first iteration)
      fireEvent.click(copyButtons[0])

      await waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalled()
      })
    })
  })
})
