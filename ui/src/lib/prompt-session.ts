import type { WorkType } from '@/components/prompt-builder/WorkTypeSelector'
import type { Message } from '@/components/prompt-builder/Conversation'

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3100'

export interface ContextResponse {
  workingDir: string
  projectName: string
}

export interface CreateSessionResponse {
  sessionId: string
}

export interface SavePromptResponse {
  path: string
}

// ============================================================================
// Prompt History Types
// ============================================================================

export interface SessionStatePayload {
  messages: Message[]
  promptDraft: string
  previewOpen: boolean
}

export interface SavePromptSessionRequest {
  id: string
  title?: string
  workType: string
  projectPath: string
  projectName: string
  content?: string
  sessionState: SessionStatePayload
}

export interface SavePromptSessionResponse {
  id: string
  updatedAt: string
}

export interface PromptSummary {
  id: string
  title?: string
  workType: string
  projectName: string
  contentPreview?: string
  createdAt: string
  updatedAt: string
}

export interface ListPromptsResponse {
  prompts: PromptSummary[]
  projects: string[]
}

export interface GetPromptResponse {
  id: string
  title?: string
  workType: string
  projectPath: string
  projectName: string
  content?: string
  sessionState: SessionStatePayload
  parentIds: string[]
  createdAt: string
  updatedAt: string
}

export interface ResolvedPromptResponse {
  id: string
  resolvedContent: string
  chain: PromptSummary[]
}

export interface ListPromptsParams {
  projectName?: string
  search?: string
  limit?: number
  offset?: number
}

export async function getContext(): Promise<ContextResponse> {
  const res = await fetch(`${API_BASE}/api/context`)
  if (!res.ok) throw new Error(`Failed to get context: ${res.statusText}`)
  return res.json()
}

export async function createPromptSession(
  workType: WorkType,
  workingDir: string
): Promise<CreateSessionResponse> {
  const res = await fetch(`${API_BASE}/api/prompt-session`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ workType, workingDir }),
  })
  if (!res.ok) throw new Error(`Failed to create session: ${res.statusText}`)
  return res.json()
}

export async function* sendPromptMessage(
  sessionId: string,
  content: string
): AsyncGenerator<string, void, unknown> {
  const res = await fetch(`${API_BASE}/api/prompt-session/${encodeURIComponent(sessionId)}/message`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ content }),
  })

  if (!res.ok) throw new Error(`Failed to send message: ${res.statusText}`)
  if (!res.body) throw new Error('No response body')

  const reader = res.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''

  while (true) {
    const { done, value } = await reader.read()
    if (done) break

    buffer += decoder.decode(value, { stream: true })
    const lines = buffer.split('\n')
    buffer = lines.pop() || ''

    for (const line of lines) {
      if (line.startsWith('data: ')) {
        const data = line.slice(6)
        if (data === '[DONE]') return
        try {
          const parsed = JSON.parse(data)
          if (parsed.error) {
            yield `__ERROR__${parsed.error}`
          } else if (parsed.content) {
            yield parsed.content
          }
          if (parsed.promptDraft) yield `__PROMPT_DRAFT__${parsed.promptDraft}`
        } catch {
          // Skip non-JSON lines
        }
      }
    }
  }
}

export async function savePrompt(
  workingDir: string,
  content: string
): Promise<SavePromptResponse> {
  const res = await fetch(`${API_BASE}/api/prompt/save`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ workingDir, content }),
  })
  if (!res.ok) throw new Error(`Failed to save prompt: ${res.statusText}`)
  return res.json()
}

// ============================================================================
// Prompt History API
// ============================================================================

export async function savePromptSession(
  request: SavePromptSessionRequest
): Promise<SavePromptSessionResponse> {
  const res = await fetch(`${API_BASE}/api/prompts`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  })
  if (!res.ok) throw new Error(`Failed to save prompt session: ${res.statusText}`)
  return res.json()
}

export async function listPrompts(
  params?: ListPromptsParams
): Promise<ListPromptsResponse> {
  const searchParams = new URLSearchParams()
  if (params?.projectName) searchParams.set('projectName', params.projectName)
  if (params?.search) searchParams.set('search', params.search)
  if (params?.limit) searchParams.set('limit', String(params.limit))
  if (params?.offset) searchParams.set('offset', String(params.offset))

  const url = `${API_BASE}/api/prompts${searchParams.toString() ? `?${searchParams}` : ''}`
  const res = await fetch(url)
  if (!res.ok) throw new Error(`Failed to list prompts: ${res.statusText}`)
  return res.json()
}

export async function getPromptById(id: string): Promise<GetPromptResponse> {
  const res = await fetch(`${API_BASE}/api/prompts/${encodeURIComponent(id)}`)
  if (!res.ok) throw new Error(`Failed to get prompt: ${res.statusText}`)
  return res.json()
}

export async function deletePrompt(id: string): Promise<void> {
  const res = await fetch(`${API_BASE}/api/prompts/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  })
  if (!res.ok) throw new Error(`Failed to delete prompt: ${res.statusText}`)
}

// ============================================================================
// Prompt Inheritance API
// ============================================================================

export async function updatePromptParents(id: string, parentIds: string[]): Promise<void> {
  const res = await fetch(`${API_BASE}/api/prompts/${encodeURIComponent(id)}/parents`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(parentIds),
  })
  if (!res.ok) throw new Error(`Failed to update prompt parents: ${res.statusText}`)
}

export async function getResolvedPrompt(id: string): Promise<ResolvedPromptResponse> {
  const res = await fetch(`${API_BASE}/api/prompts/${encodeURIComponent(id)}/resolved`)
  if (!res.ok) throw new Error(`Failed to get resolved prompt: ${res.statusText}`)
  return res.json()
}
