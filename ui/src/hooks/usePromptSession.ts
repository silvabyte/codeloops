import { useState, useEffect, useCallback, useRef } from 'react'
import type { WorkType } from '@/components/prompt-builder/WorkTypeSelector'
import type { Message } from '@/components/prompt-builder/Conversation'
import {
  getContext,
  createPromptSession,
  sendPromptMessage,
  savePrompt,
} from '@/lib/prompt-session'

export type SessionStatus = 'selecting' | 'chatting' | 'complete'

export interface PromptSession {
  id: string | null
  workType: WorkType | null
  workingDir: string
  projectName: string
  messages: Message[]
  promptDraft: string
  status: SessionStatus
  previewOpen: boolean
}

const STORAGE_KEY_PREFIX = 'codeloops-prompt-session-'

function getStorageKey(projectPath: string): string {
  return `${STORAGE_KEY_PREFIX}${projectPath.replace(/\//g, '-')}`
}

/**
 * Strip <prompt></prompt> tags from assistant messages for display.
 * The prompt content is sent separately via the promptDraft field.
 */
function stripPromptTags(content: string): string {
  // Remove <prompt>...</prompt> blocks from display
  return content.replace(/<prompt>[\s\S]*?<\/prompt>/g, '').trim()
}

function generateId(): string {
  return `msg-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
}

const defaultSession: PromptSession = {
  id: null,
  workType: null,
  workingDir: '',
  projectName: '',
  messages: [],
  promptDraft: '',
  status: 'selecting',
  previewOpen: false,
}

export function usePromptSession() {
  const [session, setSession] = useState<PromptSession>(defaultSession)
  const [isLoading, setIsLoading] = useState(false)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [contextLoading, setContextLoading] = useState(true)
  const abortControllerRef = useRef<AbortController | null>(null)

  // Load context and restore session on mount
  useEffect(() => {
    async function init() {
      try {
        const context = await getContext()
        const storageKey = getStorageKey(context.workingDir)

        // Try to restore from localStorage
        const stored = localStorage.getItem(storageKey)
        if (stored) {
          try {
            const parsed = JSON.parse(stored)
            setSession({
              ...parsed,
              workingDir: context.workingDir,
              projectName: context.projectName,
            })
          } catch {
            // Invalid stored session, start fresh
            setSession({
              ...defaultSession,
              workingDir: context.workingDir,
              projectName: context.projectName,
            })
          }
        } else {
          setSession({
            ...defaultSession,
            workingDir: context.workingDir,
            projectName: context.projectName,
          })
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to load context')
        // Use defaults if context fails - fallback to empty strings
        // since we're in the browser and don't have the server context
        setSession({
          ...defaultSession,
          workingDir: '',
          projectName: 'Unknown Project',
        })
      } finally {
        setContextLoading(false)
      }
    }
    init()
  }, [])

  // Persist session changes to localStorage
  useEffect(() => {
    if (!session.workingDir || contextLoading) return

    const storageKey = getStorageKey(session.workingDir)
    try {
      localStorage.setItem(storageKey, JSON.stringify(session))
    } catch {
      // Ignore storage errors (quota exceeded, etc.)
    }
  }, [session, contextLoading])

  const selectWorkType = useCallback(async (type: WorkType) => {
    setIsLoading(true)
    setError(null)

    try {
      const { sessionId } = await createPromptSession(type, session.workingDir)

      // Get initial AI message
      const initialMessages: Message[] = []
      let currentContent = ''

      for await (const chunk of sendPromptMessage(sessionId, '__INIT__')) {
        if (chunk.startsWith('__ERROR__')) {
          const errorMsg = chunk.slice('__ERROR__'.length)
          setError(errorMsg)
          break
        } else if (chunk.startsWith('__PROMPT_DRAFT__')) {
          const draft = chunk.slice('__PROMPT_DRAFT__'.length)
          setSession((s) => ({ ...s, promptDraft: draft }))
        } else {
          currentContent += chunk
        }
      }

      if (currentContent) {
        // Strip prompt tags for display
        const displayContent = stripPromptTags(currentContent)
        if (displayContent) {
          initialMessages.push({
            id: generateId(),
            role: 'assistant',
            content: displayContent,
          })
        }
      }

      setSession((s) => ({
        ...s,
        id: sessionId,
        workType: type,
        status: 'chatting',
        messages: initialMessages,
      }))
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to start session')
    } finally {
      setIsLoading(false)
    }
  }, [session.workingDir])

  const sendMessage = useCallback(async (content: string) => {
    if (!session.id || isLoading) return

    // Add user message immediately
    const userMessage: Message = {
      id: generateId(),
      role: 'user',
      content,
    }

    setSession((s) => ({
      ...s,
      messages: [...s.messages, userMessage],
    }))

    setIsLoading(true)
    setError(null)

    try {
      let assistantContent = ''
      const assistantId = generateId()

      // Stream AI response
      for await (const chunk of sendPromptMessage(session.id, content)) {
        if (chunk.startsWith('__ERROR__')) {
          const errorMsg = chunk.slice('__ERROR__'.length)
          setError(errorMsg)
          break
        } else if (chunk.startsWith('__PROMPT_DRAFT__')) {
          const draft = chunk.slice('__PROMPT_DRAFT__'.length)
          setSession((s) => ({ ...s, promptDraft: draft }))
        } else {
          assistantContent += chunk
          // Update message as it streams (strip <prompt> tags for display)
          const displayContent = stripPromptTags(assistantContent)
          setSession((s) => {
            const existingIdx = s.messages.findIndex((m) => m.id === assistantId)
            if (existingIdx >= 0) {
              const updated = [...s.messages]
              updated[existingIdx] = {
                ...updated[existingIdx],
                content: displayContent,
              }
              return { ...s, messages: updated }
            } else {
              return {
                ...s,
                messages: [
                  ...s.messages,
                  { id: assistantId, role: 'assistant', content: displayContent },
                ],
              }
            }
          })
        }
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to send message')
    } finally {
      setIsLoading(false)
    }
  }, [session.id, isLoading])

  const updatePromptDraft = useCallback((content: string) => {
    setSession((s) => ({ ...s, promptDraft: content }))
  }, [])

  const togglePreview = useCallback(() => {
    setSession((s) => ({ ...s, previewOpen: !s.previewOpen }))
  }, [])

  const closePreview = useCallback(() => {
    setSession((s) => ({ ...s, previewOpen: false }))
  }, [])

  const save = useCallback(async () => {
    if (!session.promptDraft || isSaving) return null

    setIsSaving(true)
    setError(null)

    try {
      const result = await savePrompt(session.workingDir, session.promptDraft)
      return result.path
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to save prompt')
      return null
    } finally {
      setIsSaving(false)
    }
  }, [session.workingDir, session.promptDraft, isSaving])

  const reset = useCallback(() => {
    // Cancel any ongoing requests
    abortControllerRef.current?.abort()

    // Clear localStorage
    if (session.workingDir) {
      const storageKey = getStorageKey(session.workingDir)
      localStorage.removeItem(storageKey)
    }

    // Reset session state
    setSession((s) => ({
      ...defaultSession,
      workingDir: s.workingDir,
      projectName: s.projectName,
    }))
    setError(null)
    setIsLoading(false)
    setIsSaving(false)
  }, [session.workingDir])

  return {
    session,
    isLoading,
    isSaving,
    error,
    contextLoading,
    selectWorkType,
    sendMessage,
    updatePromptDraft,
    togglePreview,
    closePreview,
    save,
    reset,
  }
}
