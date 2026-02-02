import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { PreviewPanel } from '../PreviewPanel'

describe('PreviewPanel', () => {
  const mockOnContentChange = vi.fn()
  const mockOnSave = vi.fn()
  const mockOnCopy = vi.fn()
  const mockOnDownload = vi.fn()

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

      expect(screen.getByText('# Test Content')).toBeInTheDocument()
    })

    it('should render action buttons', () => {
      render(<PreviewPanel {...defaultProps} />)

      expect(screen.getByText('Save')).toBeInTheDocument()
      expect(screen.getByText('Copy')).toBeInTheDocument()
      expect(screen.getByText('Download')).toBeInTheDocument()
    })

    it('should render Edit button', () => {
      render(<PreviewPanel {...defaultProps} />)

      expect(screen.getByText('Edit')).toBeInTheDocument()
    })
  })

  describe('edit mode', () => {
    it('should toggle to edit mode when clicking Edit', () => {
      render(<PreviewPanel {...defaultProps} content="Test content" />)

      fireEvent.click(screen.getByText('Edit'))

      // In edit mode, button should show "Preview"
      expect(screen.getByText('Preview')).toBeInTheDocument()
      expect(screen.getByRole('textbox')).toBeInTheDocument()
    })

    it('should toggle back to preview mode when clicking Preview', () => {
      render(<PreviewPanel {...defaultProps} content="Test content" />)

      // Enter edit mode
      fireEvent.click(screen.getByText('Edit'))
      expect(screen.getByText('Preview')).toBeInTheDocument()

      // Exit edit mode
      fireEvent.click(screen.getByText('Preview'))
      expect(screen.getByText('Edit')).toBeInTheDocument()
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

      fireEvent.click(screen.getByText('Save'))

      expect(mockOnSave).toHaveBeenCalledTimes(1)
    })

    it('should disable Save button when isSaving', () => {
      render(<PreviewPanel {...defaultProps} content="Content" isSaving={true} />)

      const saveButton = screen.getByText('Saving...')
      expect(saveButton).toBeDisabled()
    })

    it('should show "Saving..." text when isSaving', () => {
      render(<PreviewPanel {...defaultProps} content="Content" isSaving={true} />)

      expect(screen.getByText('Saving...')).toBeInTheDocument()
    })

    it('should disable Save button when no content', () => {
      render(<PreviewPanel {...defaultProps} content="" />)

      const saveButton = screen.getByText('Save')
      expect(saveButton).toBeDisabled()
    })
  })

  describe('copy action', () => {
    it('should call onCopy when clicking Copy', async () => {
      render(<PreviewPanel {...defaultProps} content="Copy me" />)

      fireEvent.click(screen.getByText('Copy'))

      // Wait for async clipboard operation
      await vi.waitFor(() => {
        expect(mockOnCopy).toHaveBeenCalledTimes(1)
      })
    })

    it('should copy content to clipboard', async () => {
      render(<PreviewPanel {...defaultProps} content="Clipboard content" />)

      fireEvent.click(screen.getByText('Copy'))

      await vi.waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalledWith('Clipboard content')
      })
    })

    it('should disable Copy button when no content', () => {
      render(<PreviewPanel {...defaultProps} content="" />)

      const copyButton = screen.getByText('Copy')
      expect(copyButton).toBeDisabled()
    })
  })

  describe('download action', () => {
    it('should call onDownload when clicking Download', () => {
      render(<PreviewPanel {...defaultProps} content="Download me" />)

      fireEvent.click(screen.getByText('Download'))

      expect(mockOnDownload).toHaveBeenCalledTimes(1)
    })

    it('should disable Download button when no content', () => {
      render(<PreviewPanel {...defaultProps} content="" />)

      const downloadButton = screen.getByText('Download')
      expect(downloadButton).toBeDisabled()
    })

    it('should create blob and trigger download', () => {
      const createElementSpy = vi.spyOn(document, 'createElement')

      render(<PreviewPanel {...defaultProps} content="Download content" />)

      fireEvent.click(screen.getByText('Download'))

      expect(URL.createObjectURL).toHaveBeenCalled()
      expect(createElementSpy).toHaveBeenCalledWith('a')
      expect(URL.revokeObjectURL).toHaveBeenCalled()

      createElementSpy.mockRestore()
    })
  })

  describe('button states', () => {
    it('should enable all buttons when content is present', () => {
      render(<PreviewPanel {...defaultProps} content="Content" />)

      expect(screen.getByText('Save')).not.toBeDisabled()
      expect(screen.getByText('Copy')).not.toBeDisabled()
      expect(screen.getByText('Download')).not.toBeDisabled()
    })

    it('should disable all action buttons when no content', () => {
      render(<PreviewPanel {...defaultProps} content="" />)

      expect(screen.getByText('Save')).toBeDisabled()
      expect(screen.getByText('Copy')).toBeDisabled()
      expect(screen.getByText('Download')).toBeDisabled()
    })
  })
})
