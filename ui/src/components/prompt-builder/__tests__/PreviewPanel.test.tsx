import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { PreviewPanel } from '../PreviewPanel'

// Mock fetchSkills
vi.mock('@/lib/prompt-session', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/prompt-session')>()
  return {
    ...actual,
    fetchSkills: vi.fn(),
  }
})

import { fetchSkills } from '@/lib/prompt-session'

const mockFetchSkills = vi.mocked(fetchSkills)

describe('PreviewPanel', () => {
  const mockOnContentChange = vi.fn()
  const mockOnSave = vi.fn()
  const mockOnCopy = vi.fn()
  const mockOnDownload = vi.fn()
  const mockOnToggleSkill = vi.fn()

  const defaultProps = {
    content: '',
    onContentChange: mockOnContentChange,
    onSave: mockOnSave,
    onCopy: mockOnCopy,
    onDownload: mockOnDownload,
    isSaving: false,
  }

  beforeEach(() => {
    vi.clearAllMocks()
    mockFetchSkills.mockResolvedValue([])
  })

  describe('rendering', () => {
    it('should render header with file name', () => {
      render(<PreviewPanel {...defaultProps} />)

      expect(screen.getByText('prompt.md')).toBeInTheDocument()
    })

    it('should render empty state message when no content', () => {
      render(<PreviewPanel {...defaultProps} />)

      expect(screen.getByText('Prompt will appear here as you chat...')).toBeInTheDocument()
    })

    it('should render content when provided', () => {
      render(<PreviewPanel {...defaultProps} content="# Test Content" />)

      expect(screen.getByRole('heading', { name: 'Test Content' })).toBeInTheDocument()
    })

    it('should render action buttons', () => {
      render(<PreviewPanel {...defaultProps} />)

      expect(screen.getByRole('button', { name: /save/i })).toBeInTheDocument()
      expect(screen.getByRole('button', { name: /copy/i })).toBeInTheDocument()
      expect(screen.getByRole('button', { name: /download/i })).toBeInTheDocument()
    })

    it('should render segmented toggle with Preview and Edit', () => {
      render(<PreviewPanel {...defaultProps} />)

      expect(screen.getByText('Preview')).toBeInTheDocument()
      expect(screen.getByText('Edit')).toBeInTheDocument()
    })
  })

  describe('edit mode', () => {
    it('should toggle to edit mode when clicking Edit', () => {
      render(<PreviewPanel {...defaultProps} content="Test content" />)

      fireEvent.click(screen.getByText('Edit'))

      expect(screen.getByRole('textbox')).toBeInTheDocument()
    })

    it('should toggle back to preview mode when clicking Preview', () => {
      render(<PreviewPanel {...defaultProps} content="Test content" />)

      // Enter edit mode
      fireEvent.click(screen.getByText('Edit'))
      expect(screen.getByRole('textbox')).toBeInTheDocument()

      // Exit edit mode
      fireEvent.click(screen.getByText('Preview'))
      expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
    })

    it('should show content in textarea when editing', () => {
      render(<PreviewPanel {...defaultProps} content="# My Prompt" />)

      fireEvent.click(screen.getByText('Edit'))

      const textarea = screen.getByRole('textbox')
      expect(textarea).toHaveValue('# My Prompt')
    })

    it('should call onContentChange when editing', () => {
      render(<PreviewPanel {...defaultProps} content="Original" />)

      fireEvent.click(screen.getByText('Edit'))
      const textarea = screen.getByRole('textbox')
      fireEvent.change(textarea, { target: { value: 'Updated content' } })

      expect(mockOnContentChange).toHaveBeenCalledWith('Updated content')
    })
  })

  describe('save action', () => {
    it('should call onSave when clicking Save', () => {
      render(<PreviewPanel {...defaultProps} content="Some content" />)

      fireEvent.click(screen.getByRole('button', { name: /save/i }))

      expect(mockOnSave).toHaveBeenCalledTimes(1)
    })

    it('should disable Save button when isSaving', () => {
      render(<PreviewPanel {...defaultProps} content="Content" isSaving={true} />)

      const saveButton = screen.getByRole('button', { name: /saving/i })
      expect(saveButton).toBeDisabled()
    })

    it('should show "Saving..." text when isSaving', () => {
      render(<PreviewPanel {...defaultProps} content="Content" isSaving={true} />)

      expect(screen.getByText('Saving...')).toBeInTheDocument()
    })

    it('should disable Save button when no content', () => {
      render(<PreviewPanel {...defaultProps} content="" />)

      const saveButton = screen.getByRole('button', { name: /save/i })
      expect(saveButton).toBeDisabled()
    })
  })

  describe('copy action', () => {
    it('should call onCopy when clicking Copy', async () => {
      render(<PreviewPanel {...defaultProps} content="Copy me" />)

      fireEvent.click(screen.getByRole('button', { name: /copy/i }))

      await vi.waitFor(() => {
        expect(mockOnCopy).toHaveBeenCalledTimes(1)
      })
    })

    it('should copy content to clipboard', async () => {
      render(<PreviewPanel {...defaultProps} content="Clipboard content" />)

      fireEvent.click(screen.getByRole('button', { name: /copy/i }))

      await vi.waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalledWith('Clipboard content')
      })
    })

    it('should disable Copy button when no content', () => {
      render(<PreviewPanel {...defaultProps} content="" />)

      const copyButton = screen.getByRole('button', { name: /copy/i })
      expect(copyButton).toBeDisabled()
    })
  })

  describe('download action', () => {
    it('should call onDownload when clicking Download', () => {
      render(<PreviewPanel {...defaultProps} content="Download me" />)

      fireEvent.click(screen.getByRole('button', { name: /download/i }))

      expect(mockOnDownload).toHaveBeenCalledTimes(1)
    })

    it('should disable Download button when no content', () => {
      render(<PreviewPanel {...defaultProps} content="" />)

      const downloadButton = screen.getByRole('button', { name: /download/i })
      expect(downloadButton).toBeDisabled()
    })

    it('should create blob and trigger download', () => {
      const createElementSpy = vi.spyOn(document, 'createElement')

      render(<PreviewPanel {...defaultProps} content="Download content" />)

      fireEvent.click(screen.getByRole('button', { name: /download/i }))

      expect(URL.createObjectURL).toHaveBeenCalled()
      expect(createElementSpy).toHaveBeenCalledWith('a')
      expect(URL.revokeObjectURL).toHaveBeenCalled()

      createElementSpy.mockRestore()
    })
  })

  describe('streaming indicator', () => {
    it('should show streaming indicator when isStreaming is true', () => {
      render(<PreviewPanel {...defaultProps} content="Content" isStreaming={true} />)

      expect(screen.getByText('Generating prompt...')).toBeInTheDocument()
    })

    it('should not show streaming indicator when isStreaming is false', () => {
      render(<PreviewPanel {...defaultProps} content="Content" isStreaming={false} />)

      expect(screen.queryByText('Generating prompt...')).not.toBeInTheDocument()
    })
  })

  describe('button states', () => {
    it('should enable all buttons when content is present', () => {
      render(<PreviewPanel {...defaultProps} content="Content" />)

      expect(screen.getByRole('button', { name: /save/i })).not.toBeDisabled()
      expect(screen.getByRole('button', { name: /copy/i })).not.toBeDisabled()
      expect(screen.getByRole('button', { name: /download/i })).not.toBeDisabled()
    })

    it('should disable all action buttons when no content', () => {
      render(<PreviewPanel {...defaultProps} content="" />)

      expect(screen.getByRole('button', { name: /save/i })).toBeDisabled()
      expect(screen.getByRole('button', { name: /copy/i })).toBeDisabled()
      expect(screen.getByRole('button', { name: /download/i })).toBeDisabled()
    })
  })

  describe('skills section', () => {
    const skillsProps = {
      ...defaultProps,
      onToggleSkill: mockOnToggleSkill,
      enabledSkills: [] as string[],
    }

    it('should render skills section when onToggleSkill is provided', () => {
      render(<PreviewPanel {...skillsProps} />)

      expect(screen.getByText('Skills')).toBeInTheDocument()
    })

    it('should not render skills section when onToggleSkill is not provided', () => {
      render(<PreviewPanel {...defaultProps} />)

      expect(screen.queryByText('Skills')).not.toBeInTheDocument()
    })

    it('should show count badge when skills are active', () => {
      render(<PreviewPanel {...skillsProps} enabledSkills={['brainstorming', 'system-design']} />)

      expect(screen.getByText('2 active')).toBeInTheDocument()
    })

    it('should not show count badge when no skills are active', () => {
      render(<PreviewPanel {...skillsProps} enabledSkills={[]} />)

      expect(screen.queryByText(/active/)).not.toBeInTheDocument()
    })

    it('should expand/collapse skills list on header click', async () => {
      mockFetchSkills.mockResolvedValue([
        { id: 'brainstorming', name: 'brainstorming', description: 'Explore ideas', sourceDir: '~/.claude/skills' },
      ])

      render(<PreviewPanel {...skillsProps} />)

      // Initially collapsed - skills list not visible
      await waitFor(() => {
        expect(screen.queryByText('Explore ideas')).not.toBeInTheDocument()
      })

      // Click to expand
      fireEvent.click(screen.getByText('Skills'))

      await waitFor(() => {
        expect(screen.getByText('brainstorming')).toBeInTheDocument()
        expect(screen.getByText('Explore ideas')).toBeInTheDocument()
      })

      // Click to collapse
      fireEvent.click(screen.getByText('Skills'))

      await waitFor(() => {
        expect(screen.queryByText('Explore ideas')).not.toBeInTheDocument()
      })
    })

    it('should call onToggleSkill when clicking a skill', async () => {
      mockFetchSkills.mockResolvedValue([
        { id: 'brainstorming', name: 'brainstorming', description: 'Explore ideas', sourceDir: '~/.claude/skills' },
      ])

      render(<PreviewPanel {...skillsProps} />)

      // Expand skills
      fireEvent.click(screen.getByText('Skills'))

      await waitFor(() => {
        expect(screen.getByText('brainstorming')).toBeInTheDocument()
      })

      // Click the skill
      fireEvent.click(screen.getByText('brainstorming'))

      expect(mockOnToggleSkill).toHaveBeenCalledWith('brainstorming')
    })

    it('should show empty state when no skills are discovered', async () => {
      mockFetchSkills.mockResolvedValue([])

      render(<PreviewPanel {...skillsProps} />)

      // Expand skills
      fireEvent.click(screen.getByText('Skills'))

      await waitFor(() => {
        expect(screen.getByText(/No skills found/)).toBeInTheDocument()
      })
    })

    it('should show loading skeleton while fetching', async () => {
      // Use a never-resolving promise to keep loading state
      let resolveSkills: (value: never[]) => void
      mockFetchSkills.mockReturnValue(new Promise((resolve) => { resolveSkills = resolve }))

      render(<PreviewPanel {...skillsProps} />)

      // Expand skills to see skeleton
      fireEvent.click(screen.getByText('Skills'))

      // Should show skeleton placeholders (animate-pulse elements)
      const pulsingElements = document.querySelectorAll('.animate-pulse')
      expect(pulsingElements.length).toBeGreaterThan(0)

      // Resolve to clean up
      resolveSkills!([])
    })
  })
})
