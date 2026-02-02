import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
  getContext,
  createPromptSession,
  sendPromptMessage,
  savePrompt,
} from '../prompt-session'

describe('prompt-session API client', () => {
  const mockFetch = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
    globalThis.fetch = mockFetch as typeof fetch
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('getContext', () => {
    it('should fetch context successfully', async () => {
      const mockResponse = {
        workingDir: '/test/project',
        projectName: 'Test Project',
      }

      mockFetch.mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockResponse),
      })

      const result = await getContext()

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/context')
      )
      expect(result).toEqual(mockResponse)
    })

    it('should throw error on failed request', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        statusText: 'Internal Server Error',
      })

      await expect(getContext()).rejects.toThrow('Failed to get context')
    })
  })

  describe('createPromptSession', () => {
    it('should create session successfully', async () => {
      const mockResponse = { sessionId: 'test-123' }

      mockFetch.mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockResponse),
      })

      const result = await createPromptSession('feature', '/test/project')

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/prompt-session'),
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ workType: 'feature', workingDir: '/test/project' }),
        }
      )
      expect(result).toEqual(mockResponse)
    })

    it('should throw error on failed request', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        statusText: 'Bad Request',
      })

      await expect(
        createPromptSession('feature', '/test')
      ).rejects.toThrow('Failed to create session')
    })
  })

  describe('sendPromptMessage', () => {
    it('should stream message chunks', async () => {
      const chunks = [
        'data: {"content":"Hello"}\n',
        'data: {"content":" World"}\n',
        'data: [DONE]\n',
      ]

      const mockReader = {
        read: vi
          .fn()
          .mockResolvedValueOnce({
            done: false,
            value: new TextEncoder().encode(chunks[0]),
          })
          .mockResolvedValueOnce({
            done: false,
            value: new TextEncoder().encode(chunks[1]),
          })
          .mockResolvedValueOnce({
            done: false,
            value: new TextEncoder().encode(chunks[2]),
          })
          .mockResolvedValueOnce({ done: true }),
      }

      mockFetch.mockResolvedValue({
        ok: true,
        body: {
          getReader: () => mockReader,
        },
      })

      const messages: string[] = []
      for await (const chunk of sendPromptMessage('test-123', 'Hello')) {
        messages.push(chunk)
      }

      expect(messages).toEqual(['Hello', ' World'])
    })

    it('should handle prompt draft in response', async () => {
      const chunk = 'data: {"promptDraft":"# Feature: Test"}\n'

      const mockReader = {
        read: vi
          .fn()
          .mockResolvedValueOnce({
            done: false,
            value: new TextEncoder().encode(chunk),
          })
          .mockResolvedValueOnce({
            done: false,
            value: new TextEncoder().encode('data: [DONE]\n'),
          })
          .mockResolvedValueOnce({ done: true }),
      }

      mockFetch.mockResolvedValue({
        ok: true,
        body: {
          getReader: () => mockReader,
        },
      })

      const messages: string[] = []
      for await (const msg of sendPromptMessage('test-123', 'Hello')) {
        messages.push(msg)
      }

      expect(messages).toContain('__PROMPT_DRAFT__# Feature: Test')
    })

    it('should throw error on failed request', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        statusText: 'Not Found',
      })

      const generator = sendPromptMessage('invalid', 'test')

      await expect(generator.next()).rejects.toThrow('Failed to send message')
    })

    it('should throw error if no response body', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        body: null,
      })

      const generator = sendPromptMessage('test-123', 'test')

      await expect(generator.next()).rejects.toThrow('No response body')
    })

    it('should handle non-JSON lines gracefully', async () => {
      const chunks = [
        'data: invalid-json\n',
        'data: {"content":"Valid"}\n',
        'data: [DONE]\n',
      ]

      const mockReader = {
        read: vi
          .fn()
          .mockResolvedValueOnce({
            done: false,
            value: new TextEncoder().encode(chunks.join('')),
          })
          .mockResolvedValueOnce({ done: true }),
      }

      mockFetch.mockResolvedValue({
        ok: true,
        body: {
          getReader: () => mockReader,
        },
      })

      const messages: string[] = []
      for await (const chunk of sendPromptMessage('test-123', 'Hello')) {
        messages.push(chunk)
      }

      // Should only include valid JSON content
      expect(messages).toEqual(['Valid'])
    })
  })

  describe('savePrompt', () => {
    it('should save prompt successfully', async () => {
      const mockResponse = { path: '/test/project/prompt.md' }

      mockFetch.mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockResponse),
      })

      const result = await savePrompt('/test/project', '# Test content')

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/prompt/save'),
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ workingDir: '/test/project', content: '# Test content' }),
        }
      )
      expect(result).toEqual(mockResponse)
    })

    it('should throw error on failed request', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        statusText: 'Permission Denied',
      })

      await expect(
        savePrompt('/test', 'content')
      ).rejects.toThrow('Failed to save prompt')
    })
  })
})
