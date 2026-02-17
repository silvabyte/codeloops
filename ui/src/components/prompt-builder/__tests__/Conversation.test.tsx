import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { Conversation, type Message } from '../Conversation'

describe('Conversation', () => {
  const mockOnSend = vi.fn()
  const defaultProps = {
    messages: [] as Message[],
    onSend: mockOnSend,
    isLoading: false,
    showTypingIndicator: false,
    disabled: false,
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('rendering', () => {
    it('should render empty state', () => {
      render(<Conversation {...defaultProps} />)

      expect(screen.getByPlaceholderText("Describe what you're building...")).toBeInTheDocument()
      expect(screen.getByText('Start a conversation to build your prompt')).toBeInTheDocument()
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

    it('should show typing indicator when showTypingIndicator is true', () => {
      render(<Conversation {...defaultProps} showTypingIndicator={true} />)

      // Typing indicator renders 3 dots with the typing-indicator class
      const indicator = document.querySelector('.typing-indicator')
      expect(indicator).toBeInTheDocument()
    })

    it('should show hint text', () => {
      render(<Conversation {...defaultProps} />)

      expect(screen.getByText('Shift+Enter for new line')).toBeInTheDocument()
    })
  })

  describe('input handling', () => {
    it('should update input value on change', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText("Describe what you're building...")
      fireEvent.change(textarea, { target: { value: 'Test message' } })

      expect(textarea).toHaveValue('Test message')
    })

    it('should call onSend with Enter', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText("Describe what you're building...")
      fireEvent.change(textarea, { target: { value: 'Test message' } })
      fireEvent.keyDown(textarea, { key: 'Enter' })

      expect(mockOnSend).toHaveBeenCalledWith('Test message')
    })

    it('should not call onSend with Shift+Enter (allows newline)', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText("Describe what you're building...")
      fireEvent.change(textarea, { target: { value: 'Test message' } })
      fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: true })

      expect(mockOnSend).not.toHaveBeenCalled()
    })

    it('should clear input after sending', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText("Describe what you're building...")
      fireEvent.change(textarea, { target: { value: 'Test message' } })
      fireEvent.keyDown(textarea, { key: 'Enter' })

      expect(textarea).toHaveValue('')
    })

    it('should not send empty messages', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText("Describe what you're building...")
      fireEvent.keyDown(textarea, { key: 'Enter' })

      expect(mockOnSend).not.toHaveBeenCalled()
    })

    it('should not send whitespace-only messages', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText("Describe what you're building...")
      fireEvent.change(textarea, { target: { value: '   ' } })
      fireEvent.keyDown(textarea, { key: 'Enter' })

      expect(mockOnSend).not.toHaveBeenCalled()
    })

    it('should trim message before sending', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText("Describe what you're building...")
      fireEvent.change(textarea, { target: { value: '  Test message  ' } })
      fireEvent.keyDown(textarea, { key: 'Enter' })

      expect(mockOnSend).toHaveBeenCalledWith('Test message')
    })

    it('should send via send button click', () => {
      render(<Conversation {...defaultProps} />)

      const textarea = screen.getByPlaceholderText("Describe what you're building...")
      fireEvent.change(textarea, { target: { value: 'Test message' } })
      fireEvent.click(screen.getByRole('button', { name: 'Send message' }))

      expect(mockOnSend).toHaveBeenCalledWith('Test message')
    })
  })

  describe('disabled state', () => {
    it('should disable textarea when disabled prop is true', () => {
      render(<Conversation {...defaultProps} disabled={true} />)

      const textarea = screen.getByPlaceholderText("Describe what you're building...")
      expect(textarea).toBeDisabled()
    })

    it('should disable textarea when isLoading is true', () => {
      render(<Conversation {...defaultProps} isLoading={true} />)

      const textarea = screen.getByPlaceholderText("Describe what you're building...")
      expect(textarea).toBeDisabled()
    })

    it('should not send message when disabled', () => {
      render(<Conversation {...defaultProps} disabled={true} />)

      const textarea = screen.getByPlaceholderText("Describe what you're building...")
      fireEvent.change(textarea, { target: { value: 'Test' } })
      fireEvent.keyDown(textarea, { key: 'Enter' })

      expect(mockOnSend).not.toHaveBeenCalled()
    })

    it('should not send message when loading', () => {
      render(<Conversation {...defaultProps} isLoading={true} />)

      const textarea = screen.getByPlaceholderText("Describe what you're building...")
      fireEvent.change(textarea, { target: { value: 'Test' } })
      fireEvent.keyDown(textarea, { key: 'Enter' })

      expect(mockOnSend).not.toHaveBeenCalled()
    })
  })

  describe('message styling', () => {
    it('should render user messages with user avatar', () => {
      const messages: Message[] = [
        { id: '1', role: 'user', content: 'User message' },
      ]

      render(<Conversation {...defaultProps} messages={messages} />)

      // User messages have ml-auto for right alignment
      const messageRow = screen.getByText('User message').closest('.flex.gap-3')
      expect(messageRow).toHaveClass('ml-auto')
    })

    it('should render assistant messages with bot avatar', () => {
      const messages: Message[] = [
        { id: '1', role: 'assistant', content: 'Assistant message' },
      ]

      render(<Conversation {...defaultProps} messages={messages} />)

      // Assistant messages have mr-auto for left alignment
      const messageRow = screen.getByText('Assistant message').closest('.flex.gap-3')
      expect(messageRow).toHaveClass('mr-auto')
    })
  })
})
