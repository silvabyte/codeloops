import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act, waitFor } from '@testing-library/react'
import { usePromptSession } from '../usePromptSession'
import * as promptSessionApi from '@/lib/prompt-session'

// Mock the API module
vi.mock('@/lib/prompt-session', () => ({
  getContext: vi.fn(),
  createPromptSession: vi.fn(),
  sendPromptMessage: vi.fn(),
  savePrompt: vi.fn(),
}))

const mockGetContext = vi.mocked(promptSessionApi.getContext)
const mockCreatePromptSession = vi.mocked(promptSessionApi.createPromptSession)
const mockSendPromptMessage = vi.mocked(promptSessionApi.sendPromptMessage)
const mockSavePrompt = vi.mocked(promptSessionApi.savePrompt)

describe('usePromptSession', () => {
  const mockContext = {
    workingDir: '/test/project',
    projectName: 'Test Project',
  }

  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    mockGetContext.mockResolvedValue(mockContext)
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('initial state', () => {
    it('should have correct initial state', async () => {
      const { result } = renderHook(() => usePromptSession())

      // Initially loading context
      expect(result.current.contextLoading).toBe(true)

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      expect(result.current.session.status).toBe('selecting')
      expect(result.current.session.workType).toBeNull()
      expect(result.current.session.messages).toEqual([])
      expect(result.current.session.promptDraft).toBe('')
      expect(result.current.session.previewOpen).toBe(false)
      expect(result.current.session.workingDir).toBe('/test/project')
      expect(result.current.session.projectName).toBe('Test Project')
    })

    it('should load context on mount', async () => {
      renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(mockGetContext).toHaveBeenCalledTimes(1)
      })
    })
  })

  describe('work type selection', () => {
    it('should select work type and start session', async () => {
      const mockSessionId = 'test-session-123'
      mockCreatePromptSession.mockResolvedValue({ sessionId: mockSessionId })

      // Mock async generator for sendPromptMessage
      async function* mockMessages() {
        yield 'Hello, '
        yield 'how can I help?'
      }
      mockSendPromptMessage.mockReturnValue(mockMessages())

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      await act(async () => {
        await result.current.selectWorkType('feature')
      })

      expect(mockCreatePromptSession).toHaveBeenCalledWith('feature', '/test/project')
      expect(result.current.session.workType).toBe('feature')
      expect(result.current.session.status).toBe('chatting')
      expect(result.current.session.id).toBe(mockSessionId)
    })

    it('should handle error during work type selection', async () => {
      mockCreatePromptSession.mockRejectedValue(new Error('Network error'))

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      await act(async () => {
        await result.current.selectWorkType('feature')
      })

      expect(result.current.error).toBe('Network error')
      expect(result.current.session.status).toBe('selecting')
    })
  })

  describe('message sending', () => {
    it('should send user message and receive response', async () => {
      const mockSessionId = 'test-session-123'
      mockCreatePromptSession.mockResolvedValue({ sessionId: mockSessionId })

      // Initial message
      async function* initMessages() {
        yield 'Welcome!'
      }
      mockSendPromptMessage.mockReturnValueOnce(initMessages())

      // User message response
      async function* responseMessages() {
        yield 'Great question! '
        yield 'Here is my answer.'
        yield '__PROMPT_DRAFT__# Feature: Test'
      }

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      // Start session
      await act(async () => {
        await result.current.selectWorkType('feature')
      })

      // Mock for user message
      mockSendPromptMessage.mockReturnValueOnce(responseMessages())

      // Send message
      await act(async () => {
        await result.current.sendMessage('Test message')
      })

      // Should have user message and assistant response
      expect(result.current.session.messages.length).toBeGreaterThanOrEqual(2)
      expect(result.current.session.promptDraft).toBe('# Feature: Test')
    })

    it('should not send message if no session', async () => {
      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      await act(async () => {
        await result.current.sendMessage('Test')
      })

      expect(mockSendPromptMessage).not.toHaveBeenCalled()
    })
  })

  describe('prompt draft editing', () => {
    it('should update prompt draft when in ready state', async () => {
      const mockSessionId = 'test-session-123'
      mockCreatePromptSession.mockResolvedValue({ sessionId: mockSessionId })

      async function* mockMessages() {
        yield 'Welcome!'
      }
      mockSendPromptMessage.mockReturnValue(mockMessages())

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      // First establish a session to get to ready state
      await act(async () => {
        await result.current.selectWorkType('feature')
      })

      act(() => {
        result.current.updatePromptDraft('# New content')
      })

      expect(result.current.session.promptDraft).toBe('# New content')
    })

    it('should not update prompt draft when not in ready state', async () => {
      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      // Try to update without a session (selecting_work_type state)
      act(() => {
        result.current.updatePromptDraft('# New content')
      })

      // Should remain empty because we're not in ready state
      expect(result.current.session.promptDraft).toBe('')
    })
  })

  describe('preview toggle', () => {
    it('should toggle preview panel when in ready state', async () => {
      const mockSessionId = 'test-session-123'
      mockCreatePromptSession.mockResolvedValue({ sessionId: mockSessionId })

      async function* mockMessages() {
        yield 'Welcome!'
      }
      mockSendPromptMessage.mockReturnValue(mockMessages())

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      // First establish a session to get to ready state
      await act(async () => {
        await result.current.selectWorkType('feature')
      })

      expect(result.current.session.previewOpen).toBe(false)

      act(() => {
        result.current.togglePreview()
      })

      expect(result.current.session.previewOpen).toBe(true)

      act(() => {
        result.current.togglePreview()
      })

      expect(result.current.session.previewOpen).toBe(false)
    })

    it('should close preview when in ready state', async () => {
      const mockSessionId = 'test-session-123'
      mockCreatePromptSession.mockResolvedValue({ sessionId: mockSessionId })

      async function* mockMessages() {
        yield 'Welcome!'
      }
      mockSendPromptMessage.mockReturnValue(mockMessages())

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      // First establish a session to get to ready state
      await act(async () => {
        await result.current.selectWorkType('feature')
      })

      act(() => {
        result.current.togglePreview()
      })

      expect(result.current.session.previewOpen).toBe(true)

      act(() => {
        result.current.closePreview()
      })

      expect(result.current.session.previewOpen).toBe(false)
    })

    it('should not toggle preview when not in ready state', async () => {
      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      // Try to toggle without a session (selecting_work_type state)
      act(() => {
        result.current.togglePreview()
      })

      // Should remain false because we're not in ready state
      expect(result.current.session.previewOpen).toBe(false)
    })
  })

  describe('localStorage persistence', () => {
    it('should save session to localStorage', async () => {
      mockCreatePromptSession.mockResolvedValue({ sessionId: 'test-123' })

      async function* mockMessages() {
        yield 'Test'
      }
      mockSendPromptMessage.mockReturnValue(mockMessages())

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      await act(async () => {
        await result.current.selectWorkType('defect')
      })

      expect(localStorage.setItem).toHaveBeenCalled()
    })

    it('should restore session from localStorage', async () => {
      const storedSession = {
        id: 'stored-123',
        workType: 'feature',
        messages: [{ id: '1', role: 'assistant', content: 'Hello' }],
        promptDraft: '# Test',
        status: 'chatting',
        previewOpen: true,
      }

      vi.mocked(localStorage.getItem).mockReturnValue(JSON.stringify(storedSession))

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      expect(result.current.session.id).toBe('stored-123')
      expect(result.current.session.workType).toBe('feature')
      expect(result.current.session.status).toBe('chatting')
      expect(result.current.session.previewOpen).toBe(true)
    })

    it('should handle invalid localStorage data', async () => {
      vi.mocked(localStorage.getItem).mockReturnValue('invalid json')

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      // Should fall back to default state
      expect(result.current.session.status).toBe('selecting')
    })
  })

  describe('reset functionality', () => {
    it('should reset session to initial state', async () => {
      mockCreatePromptSession.mockResolvedValue({ sessionId: 'test-123' })

      async function* mockMessages() {
        yield 'Test'
      }
      mockSendPromptMessage.mockReturnValue(mockMessages())

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      await act(async () => {
        await result.current.selectWorkType('feature')
      })

      expect(result.current.session.status).toBe('chatting')

      act(() => {
        result.current.reset()
      })

      expect(result.current.session.status).toBe('selecting')
      expect(result.current.session.workType).toBeNull()
      expect(result.current.session.messages).toEqual([])
      expect(result.current.session.promptDraft).toBe('')
      expect(localStorage.removeItem).toHaveBeenCalled()
    })
  })

  describe('save functionality', () => {
    it('should save prompt to disk when in ready state', async () => {
      const mockPath = '/test/project/prompt.md'
      mockSavePrompt.mockResolvedValue({ path: mockPath })
      mockCreatePromptSession.mockResolvedValue({ sessionId: 'test-123' })

      async function* mockMessages() {
        yield 'Welcome!'
        yield '__PROMPT_DRAFT__# Test prompt'
      }
      mockSendPromptMessage.mockReturnValue(mockMessages())

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      // First establish a session to get to ready state with a prompt draft
      await act(async () => {
        await result.current.selectWorkType('feature')
      })

      let savedPath: string | null = null
      await act(async () => {
        savedPath = await result.current.save()
      })

      expect(mockSavePrompt).toHaveBeenCalledWith('/test/project', '# Test prompt')
      expect(savedPath).toBe(mockPath)
    })

    it('should not save if not in ready state', async () => {
      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      let savedPath: string | null = null
      await act(async () => {
        savedPath = await result.current.save()
      })

      expect(mockSavePrompt).not.toHaveBeenCalled()
      expect(savedPath).toBeNull()
    })

    it('should not save if prompt draft is empty', async () => {
      mockCreatePromptSession.mockResolvedValue({ sessionId: 'test-123' })

      async function* mockMessages() {
        yield 'Welcome!'
        // No __PROMPT_DRAFT__ chunk, so promptDraft remains empty
      }
      mockSendPromptMessage.mockReturnValue(mockMessages())

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      await act(async () => {
        await result.current.selectWorkType('feature')
      })

      let savedPath: string | null = null
      await act(async () => {
        savedPath = await result.current.save()
      })

      expect(mockSavePrompt).not.toHaveBeenCalled()
      expect(savedPath).toBeNull()
    })

    it('should handle save error', async () => {
      mockSavePrompt.mockRejectedValue(new Error('Permission denied'))
      mockCreatePromptSession.mockResolvedValue({ sessionId: 'test-123' })

      async function* mockMessages() {
        yield 'Welcome!'
        yield '__PROMPT_DRAFT__# Test'
      }
      mockSendPromptMessage.mockReturnValue(mockMessages())

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      await act(async () => {
        await result.current.selectWorkType('feature')
      })

      await act(async () => {
        await result.current.save()
      })

      expect(result.current.error).toBe('Permission denied')
    })
  })

  describe('context loading error', () => {
    it('should handle context loading error gracefully', async () => {
      mockGetContext.mockRejectedValue(new Error('Failed to fetch'))

      const { result } = renderHook(() => usePromptSession())

      await waitFor(() => {
        expect(result.current.contextLoading).toBe(false)
      })

      expect(result.current.error).toBe('Failed to fetch')
      expect(result.current.session.projectName).toBe('Unknown Project')
    })
  })
})
