import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { Conversation, type Message } from '../Conversation'

describe('Conversation', () => {
  const mockOnSend = vi.fn()
  const defaultProps = {
    messages: [] as Message[],
    onSend: mockOnSend,
    isLoading: false,
    disabled: false,
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('rendering', () => {
    it('should render empty state', () => {
      render(<Conversation {...defaultProps} />)

      expect(screen.getByPlaceholderText('Type here...')).toBeInTheDocument()
    })

    it('should render user messages', () => {
      const messages: Message[] = [
        { id: '1', role: 'user', content: 'Hello there' },
      ]

      render(<Conversation {...defaultProps} messages={messages} />)

      expect(screen.getByText('Hello there')).toBeInTheDocument()
    })

    it('should render assistant messages', () => {
      const messages: Message[] = [
        { id: '1', role: 'assistant', content: 'Hi, how can I help?' },
      ]

      render(<Conversation {...defaultProps} messages={messages} />)

      expect(screen.getByText('Hi, how can I help?')).toBeInTheDocument()
    })

    it('should render multiple messages', () => {
      const messages: Message[] = [
        { id: '1', role: 'assistant', content: 'Welcome!' },
        { id: '2', role: 'user', content: 'Thanks' },
        { id: '3', role: 'assistant', content: 'How can I help?' },
      ]

      render(<Conversation {...defaultProps} messages={messages} />)

      expect(screen.getByText('Welcome!')).toBeInTheDocument()
      expect(screen.getByText('Thanks')).toBeInTheDocument()
      expect(screen.getByText('How can I help?')).toBeInTheDocument()
    })

    it('should show loading indicator when isLoading', () => {
      render(<Conversation {...defaultProps} isLoading={true} />)

      expect(screen.getByText('...')).toBeInTheDocument()
    })

    it('should show keyboard shortcut hint', () => {
      render(<Conversation {...defaultProps} />)

      // Look for either Mac or Windows shortcut
      const hint = screen.getByText(/\+↵/)
      expect(hint).toBeInTheDocument()
    })
  })

  describe('input handling', () => {
    it('should update input value on change', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText('Type here...')
      fireEvent.change(textarea, { target: { value: 'Test message' } })

      expect(textarea).toHaveValue('Test message')
    })

    it('should call onSend with Cmd+Enter', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText('Type here...')
      fireEvent.change(textarea, { target: { value: 'Test message' } })
      fireEvent.keyDown(textarea, { key: 'Enter', metaKey: true })

      expect(mockOnSend).toHaveBeenCalledWith('Test message')
    })

    it('should call onSend with Ctrl+Enter', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText('Type here...')
      fireEvent.change(textarea, { target: { value: 'Test message' } })
      fireEvent.keyDown(textarea, { key: 'Enter', ctrlKey: true })

      expect(mockOnSend).toHaveBeenCalledWith('Test message')
    })

    it('should not call onSend with just Enter', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText('Type here...')
      fireEvent.change(textarea, { target: { value: 'Test message' } })
      fireEvent.keyDown(textarea, { key: 'Enter' })

      expect(mockOnSend).not.toHaveBeenCalled()
    })

    it('should clear input after sending', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText('Type here...')
      fireEvent.change(textarea, { target: { value: 'Test message' } })
      fireEvent.keyDown(textarea, { key: 'Enter', metaKey: true })

      expect(textarea).toHaveValue('')
    })

    it('should not send empty messages', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText('Type here...')
      fireEvent.keyDown(textarea, { key: 'Enter', metaKey: true })

      expect(mockOnSend).not.toHaveBeenCalled()
    })

    it('should not send whitespace-only messages', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText('Type here...')
      fireEvent.change(textarea, { target: { value: '   ' } })
      fireEvent.keyDown(textarea, { key: 'Enter', metaKey: true })

      expect(mockOnSend).not.toHaveBeenCalled()
    })

    it('should trim message before sending', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText('Type here...')
      fireEvent.change(textarea, { target: { value: '  Test message  ' } })
      fireEvent.keyDown(textarea, { key: 'Enter', metaKey: true })

      expect(mockOnSend).toHaveBeenCalledWith('Test message')
    })
  })

  describe('disabled state', () => {
    it('should disable textarea when disabled prop is true', () => {
      render(<Conversation {...defaultProps} disabled={true} />)

      const textarea = screen.getByPlaceholderText('Type here...')
      expect(textarea).toBeDisabled()
    })

    it('should disable textarea when isLoading is true', () => {
      render(<Conversation {...defaultProps} isLoading={true} />)

      const textarea = screen.getByPlaceholderText('Type here...')
      expect(textarea).toBeDisabled()
    })

    it('should not send message when disabled', () => {
      render(<Conversation {...defaultProps} disabled={true} />)

      const textarea = screen.getByPlaceholderText('Type here...')
      fireEvent.change(textarea, { target: { value: 'Test' } })
      fireEvent.keyDown(textarea, { key: 'Enter', metaKey: true })

      expect(mockOnSend).not.toHaveBeenCalled()
    })

    it('should not send message when loading', () => {
      render(<Conversation {...defaultProps} isLoading={true} />)

      const textarea = screen.getByPlaceholderText('Type here...')
      fireEvent.change(textarea, { target: { value: 'Test' } })
      fireEvent.keyDown(textarea, { key: 'Enter', metaKey: true })

      expect(mockOnSend).not.toHaveBeenCalled()
    })
  })

  describe('message styling', () => {
    it('should apply correct alignment for user messages', () => {
      const messages: Message[] = [
        { id: '1', role: 'user', content: 'User message' },
      ]

      render(<Conversation {...defaultProps} messages={messages} />)

      const messageContainer = screen.getByText('User message').parentElement
      expect(messageContainer).toHaveClass('ml-auto')
    })

    it('should apply correct alignment for assistant messages', () => {
      const messages: Message[] = [
        { id: '1', role: 'assistant', content: 'Assistant message' },
      ]

      render(<Conversation {...defaultProps} messages={messages} />)

      const messageContainer = screen.getByText('Assistant message').parentElement
      expect(messageContainer).toHaveClass('mr-auto')
    })
  })
})
