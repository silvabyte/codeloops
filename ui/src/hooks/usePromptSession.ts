import { useState, useEffect, useCallback, useRef } from 'react'
import type { WorkType } from '@/components/prompt-builder/WorkTypeSelector'
import type { Message } from '@/components/prompt-builder/Conversation'
import {
  getContext,
  createPromptSession,
  sendPromptMessage,
  savePrompt,
  savePromptSession,
  getPromptById,
  updatePromptParents,
} from '@/lib/prompt-session'

/**
 * Discriminated union for prompt session state machine.
 *
 * Each state maps to exactly one UI representation, making impossible states unrepresentable.
 */
export type PromptSessionState =
  | { status: 'loading_context' }
  | { status: 'selecting_work_type'; workingDir: string; projectName: string }
  | { status: 'creating_session'; workingDir: string; projectName: string; workType: WorkType }
  | { status: 'awaiting_agent'; workingDir: string; projectName: string; workType: WorkType; sessionId: string }
  | { status: 'streaming'; workingDir: string; projectName: string; workType: WorkType; sessionId: string; messages: Message[]; promptDraft: string; parentIds: string[] }
  | { status: 'ready'; workingDir: string; projectName: string; workType: WorkType; sessionId: string; messages: Message[]; promptDraft: string; parentIds: string[] }
  | { status: 'error'; workingDir: string; projectName: string; error: string; previousState?: PromptSessionState }

export interface PromptSession {
  id: string | null
  workType: WorkType | null
  workingDir: string
  projectName: string
  messages: Message[]
  promptDraft: string
  status: 'selecting' | 'chatting' | 'complete'
  parentIds: string[]
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
  return content.replace(/<prompt>[\s\S]*?<\/prompt>/g, '').trim()
}

function generateId(): string {
  return `msg-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
}

/**
 * Derive legacy PromptSession shape from state machine for backwards compatibility with components.
 */
function deriveSession(state: PromptSessionState): PromptSession {
  switch (state.status) {
    case 'loading_context':
      return {
        id: null,
        workType: null,
        workingDir: '',
        projectName: '',
        messages: [],
        promptDraft: '',
        status: 'selecting',
        parentIds: [],
      }
    case 'selecting_work_type':
      return {
        id: null,
        workType: null,
        workingDir: state.workingDir,
        projectName: state.projectName,
        messages: [],
        promptDraft: '',
        status: 'selecting',
        parentIds: [],
      }
    case 'creating_session':
    case 'awaiting_agent':
      return {
        id: state.status === 'awaiting_agent' ? state.sessionId : null,
        workType: state.workType,
        workingDir: state.workingDir,
        projectName: state.projectName,
        messages: [],
        promptDraft: '',
        status: 'chatting',
        parentIds: [],
      }
    case 'streaming':
      return {
        id: state.sessionId,
        workType: state.workType,
        workingDir: state.workingDir,
        projectName: state.projectName,
        messages: state.messages,
        promptDraft: state.promptDraft,
        status: 'chatting',
        parentIds: state.parentIds,
      }
    case 'ready':
      return {
        id: state.sessionId,
        workType: state.workType,
        workingDir: state.workingDir,
        projectName: state.projectName,
        messages: state.messages,
        promptDraft: state.promptDraft,
        status: 'chatting',
        parentIds: state.parentIds,
      }
    case 'error':
      // Preserve what we can from the previous state
      if (state.previousState && state.previousState.status !== 'error' && state.previousState.status !== 'loading_context') {
        const prev = deriveSession(state.previousState)
        return prev
      }
      return {
        id: null,
        workType: null,
        workingDir: state.workingDir,
        projectName: state.projectName,
        messages: [],
        promptDraft: '',
        status: 'selecting',
        parentIds: [],
      }
  }
}

/**
 * Derive UI-relevant flags from state machine.
 */
function deriveFlags(state: PromptSessionState) {
  return {
    isLoading: state.status === 'creating_session' || state.status === 'awaiting_agent' || state.status === 'streaming',
    isCreatingSession: state.status === 'creating_session',
    isAwaitingAgent: state.status === 'awaiting_agent',
    isStreaming: state.status === 'streaming',
    contextLoading: state.status === 'loading_context',
    error: state.status === 'error' ? state.error : null,
  }
}

interface StoredSession {
  id: string | null
  workType: WorkType | null
  messages: Message[]
  promptDraft: string
}

export function usePromptSession() {
  const [state, setState] = useState<PromptSessionState>({ status: 'loading_context' })
  const [isSaving, setIsSaving] = useState(false)
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
            const parsed: StoredSession = JSON.parse(stored)
            if (parsed.id && parsed.workType) {
              // Restore to ready state
              setState({
                status: 'ready',
                workingDir: context.workingDir,
                projectName: context.projectName,
                workType: parsed.workType,
                sessionId: parsed.id,
                messages: parsed.messages || [],
                promptDraft: parsed.promptDraft || '',
                parentIds: [],
              })
              return
            }
          } catch {
            // Invalid stored session, start fresh
          }
        }

        setState({
          status: 'selecting_work_type',
          workingDir: context.workingDir,
          projectName: context.projectName,
        })
      } catch (e) {
        setState({
          status: 'error',
          workingDir: '',
          projectName: 'Unknown Project',
          error: e instanceof Error ? e.message : 'Failed to load context',
        })
      }
    }
    init()
  }, [])

  // NOTE: Auto-save to backend is no longer needed here.
  // The backend now persists messages immediately after each exchange.
  // We only keep localStorage as a backup for immediate recovery on page refresh.

  // Persist to localStorage as backup (for immediate recovery)
  useEffect(() => {
    if (state.status === 'loading_context') return

    const workingDir = 'workingDir' in state ? state.workingDir : ''
    if (!workingDir) return

    const storageKey = getStorageKey(workingDir)

    // Only persist ready state
    if (state.status === 'ready') {
      try {
        const toStore: StoredSession = {
          id: state.sessionId,
          workType: state.workType,
          messages: state.messages,
          promptDraft: state.promptDraft,
        }
        localStorage.setItem(storageKey, JSON.stringify(toStore))
      } catch {
        // Ignore storage errors
      }
    }
  }, [state])

  const selectWorkType = useCallback(async (type: WorkType) => {
    // Guard: only transition from selecting_work_type
    if (state.status !== 'selecting_work_type') return

    const { workingDir, projectName } = state

    // Immediately transition to creating_session (UI shows chat view with typing indicator)
    setState({
      status: 'creating_session',
      workingDir,
      projectName,
      workType: type,
    })

    try {
      const { sessionId } = await createPromptSession(type, workingDir)

      // Transition to awaiting_agent
      setState({
        status: 'awaiting_agent',
        workingDir,
        projectName,
        workType: type,
        sessionId,
      })

      // Stream initial AI message
      let currentContent = ''
      let currentDraft = ''

      for await (const chunk of sendPromptMessage(sessionId, '__INIT__')) {
        if (chunk.startsWith('__ERROR__')) {
          const errorMsg = chunk.slice('__ERROR__'.length)
          setState({
            status: 'error',
            workingDir,
            projectName,
            error: errorMsg,
            previousState: {
              status: 'awaiting_agent',
              workingDir,
              projectName,
              workType: type,
              sessionId,
            },
          })
          return
        } else if (chunk.startsWith('__PROMPT_DRAFT__')) {
          currentDraft = chunk.slice('__PROMPT_DRAFT__'.length)
        } else {
          currentContent += chunk

          // Transition to streaming on first content
          const displayContent = stripPromptTags(currentContent)
          if (displayContent) {
            setState({
              status: 'streaming',
              workingDir,
              projectName,
              workType: type,
              sessionId,
              messages: [{
                id: generateId(),
                role: 'assistant',
                content: displayContent,
              }],
              promptDraft: currentDraft,
              parentIds: [],
            })
          }
        }
      }

      // Transition to ready
      const displayContent = stripPromptTags(currentContent)
      setState({
        status: 'ready',
        workingDir,
        projectName,
        workType: type,
        sessionId,
        messages: displayContent ? [{
          id: generateId(),
          role: 'assistant',
          content: displayContent,
        }] : [],
        promptDraft: currentDraft,
        parentIds: [],
      })
    } catch (e) {
      setState({
        status: 'error',
        workingDir,
        projectName,
        error: e instanceof Error ? e.message : 'Failed to start session',
        previousState: {
          status: 'selecting_work_type',
          workingDir,
          projectName,
        },
      })
    }
  }, [state])

  const sendMessage = useCallback(async (content: string) => {
    // Guard: only send from ready state
    if (state.status !== 'ready') return

    const { workingDir, projectName, workType, sessionId, messages, promptDraft, parentIds } = state

    // Add user message immediately
    const userMessage: Message = {
      id: generateId(),
      role: 'user',
      content,
    }

    const updatedMessages = [...messages, userMessage]

    // Transition to streaming with user message
    setState({
      status: 'streaming',
      workingDir,
      projectName,
      workType,
      sessionId,
      messages: updatedMessages,
      promptDraft,
      parentIds,
    })

    try {
      let assistantContent = ''
      const assistantId = generateId()
      let currentDraft = promptDraft

      for await (const chunk of sendPromptMessage(sessionId, content)) {
        if (chunk.startsWith('__ERROR__')) {
          const errorMsg = chunk.slice('__ERROR__'.length)
          setState({
            status: 'error',
            workingDir,
            projectName,
            error: errorMsg,
            previousState: {
              status: 'ready',
              workingDir,
              projectName,
              workType,
              sessionId,
              messages: updatedMessages,
              promptDraft: currentDraft,
              parentIds,
            },
          })
          return
        } else if (chunk.startsWith('__PROMPT_DRAFT__')) {
          currentDraft = chunk.slice('__PROMPT_DRAFT__'.length)
        } else {
          assistantContent += chunk
          const displayContent = stripPromptTags(assistantContent)

          // Update streaming state with assistant response
          setState((prev) => {
            if (prev.status !== 'streaming') return prev

            const existingIdx = prev.messages.findIndex((m) => m.id === assistantId)
            let newMessages: Message[]
            if (existingIdx >= 0) {
              newMessages = [...prev.messages]
              newMessages[existingIdx] = { ...newMessages[existingIdx], content: displayContent }
            } else {
              newMessages = [
                ...prev.messages,
                { id: assistantId, role: 'assistant', content: displayContent },
              ]
            }

            return {
              ...prev,
              messages: newMessages,
              promptDraft: currentDraft,
            }
          })
        }
      }

      // Transition to ready
      const displayContent = stripPromptTags(assistantContent)
      setState((prev) => {
        if (prev.status !== 'streaming') return prev

        const existingIdx = prev.messages.findIndex((m) => m.id === assistantId)
        let finalMessages: Message[]
        if (existingIdx >= 0) {
          finalMessages = [...prev.messages]
          finalMessages[existingIdx] = { ...finalMessages[existingIdx], content: displayContent }
        } else if (displayContent) {
          finalMessages = [
            ...prev.messages,
            { id: assistantId, role: 'assistant', content: displayContent },
          ]
        } else {
          finalMessages = prev.messages
        }

        return {
          status: 'ready',
          workingDir: prev.workingDir,
          projectName: prev.projectName,
          workType: prev.workType,
          sessionId: prev.sessionId,
          messages: finalMessages,
          promptDraft: currentDraft,
          parentIds: prev.parentIds,
        }
      })
    } catch (e) {
      setState({
        status: 'error',
        workingDir,
        projectName,
        error: e instanceof Error ? e.message : 'Failed to send message',
        previousState: {
          status: 'ready',
          workingDir,
          projectName,
          workType,
          sessionId,
          messages: updatedMessages,
          promptDraft,
          parentIds,
        },
      })
    }
  }, [state])

  const updatePromptDraft = useCallback((content: string) => {
    setState((prev) => {
      if (prev.status === 'ready') {
        return { ...prev, promptDraft: content }
      }
      if (prev.status === 'streaming') {
        return { ...prev, promptDraft: content }
      }
      return prev
    })
  }, [])


  const setParentIds = useCallback(
    async (newParentIds: string[]) => {
      if (state.status !== 'ready' && state.status !== 'streaming') return

      const sessionId = state.sessionId

      // Optimistically update local state
      setState((prev) => {
        if (prev.status === 'ready' || prev.status === 'streaming') {
          return { ...prev, parentIds: newParentIds }
        }
        return prev
      })

      // Persist to backend
      try {
        await updatePromptParents(sessionId, newParentIds)
      } catch (e) {
        console.error('Failed to update parent IDs:', e)
        // Revert on error (could add better error handling)
      }
    },
    [state]
  )

  const save = useCallback(async () => {
    if (state.status !== 'ready' || isSaving) return null

    const { workingDir, promptDraft } = state
    if (!promptDraft) return null

    setIsSaving(true)

    try {
      const result = await savePrompt(workingDir, promptDraft)
      return result.path
    } catch (e) {
      setState((prev) => ({
        status: 'error',
        workingDir: 'workingDir' in prev ? prev.workingDir : '',
        projectName: 'projectName' in prev ? prev.projectName : '',
        error: e instanceof Error ? e.message : 'Failed to save prompt',
        previousState: prev,
      }))
      return null
    } finally {
      setIsSaving(false)
    }
  }, [state, isSaving])

  const reset = useCallback(() => {
    abortControllerRef.current?.abort()

    const workingDir = 'workingDir' in state ? state.workingDir : ''
    const projectName = 'projectName' in state ? state.projectName : ''

    if (workingDir) {
      const storageKey = getStorageKey(workingDir)
      localStorage.removeItem(storageKey)
    }

    setState({
      status: 'selecting_work_type',
      workingDir,
      projectName,
    })
  }, [state])

  const clearError = useCallback(() => {
    if (state.status === 'error' && state.previousState) {
      setState(state.previousState)
    } else if (state.status === 'error') {
      setState({
        status: 'selecting_work_type',
        workingDir: state.workingDir,
        projectName: state.projectName,
      })
    }
  }, [state])

  // Start a new prompt (save current first, then reset)
  const newPrompt = useCallback(async () => {
    if (state.status !== 'ready') return

    const { sessionId, workType, workingDir, projectName, messages, promptDraft } = state

    // Save current session to history before starting new one
    try {
      const firstUserMessage = messages.find((m) => m.role === 'user')
      const title = firstUserMessage?.content.slice(0, 50) ||
        (promptDraft ? promptDraft.split('\n')[0].replace(/^#\s*/, '').slice(0, 50) : undefined)

      await savePromptSession({
        id: sessionId,
        title,
        workType,
        projectPath: workingDir,
        projectName,
        content: promptDraft || undefined,
        sessionState: {
          messages,
          promptDraft,
        },
      })
    } catch (e) {
      console.error('Failed to save session before new prompt:', e)
    }

    // Clear localStorage
    if (workingDir) {
      const storageKey = getStorageKey(workingDir)
      localStorage.removeItem(storageKey)
    }

    // Reset to work type selection
    setState({
      status: 'selecting_work_type',
      workingDir,
      projectName,
    })
  }, [state])

  // Load a prompt from history
  const loadPrompt = useCallback(async (promptId: string) => {
    const workingDir = 'workingDir' in state ? state.workingDir : ''
    const projectName = 'projectName' in state ? state.projectName : ''

    try {
      const response = await getPromptById(promptId)

      // Restore to ready state with loaded session
      setState({
        status: 'ready',
        workingDir: response.projectPath,
        projectName: response.projectName,
        workType: response.workType as WorkType,
        sessionId: response.id,
        messages: response.sessionState.messages,
        promptDraft: response.sessionState.promptDraft,
        parentIds: response.parentIds || [],
      })
    } catch (e) {
      setState({
        status: 'error',
        workingDir,
        projectName,
        error: e instanceof Error ? e.message : 'Failed to load prompt',
        previousState: state.status !== 'error' ? state : undefined,
      })
    }
  }, [state])

  // Derive legacy shapes for backwards compatibility
  const session = deriveSession(state)
  const flags = deriveFlags(state)

  return {
    // State machine
    state,
    // Legacy session shape (for components)
    session,
    // Derived flags
    isLoading: flags.isLoading,
    isSaving,
    error: flags.error,
    contextLoading: flags.contextLoading,
    // Granular loading states
    isCreatingSession: flags.isCreatingSession,
    isAwaitingAgent: flags.isAwaitingAgent,
    isStreaming: flags.isStreaming,
    // Actions
    selectWorkType,
    sendMessage,
    updatePromptDraft,
    setParentIds,
    save,
    reset,
    clearError,
    newPrompt,
    loadPrompt,
  }
}
