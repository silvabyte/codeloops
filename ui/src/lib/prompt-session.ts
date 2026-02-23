import type { WorkType } from '@/components/prompt-builder/WorkTypeSelector'
import type { Message } from '@/components/prompt-builder/Conversation'
import type { ProjectRecord } from '@/types/project'

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3100'

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
  enabledSkills: string[]
}

// ============================================================================
// Skills Types
// ============================================================================

export interface SkillInfo {
  id: string
  name: string
  description: string
  sourceDir: string
}

export interface ListSkillsResponse {
  skills: SkillInfo[]
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

/** Get project context from a registered project. */
export async function getProjectContext(projectId: string): Promise<ProjectRecord> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/context`)
  if (!res.ok) throw new Error(`Failed to get project context: ${res.statusText}`)
  return res.json()
}

export async function createPromptSession(
  projectId: string,
  workType: WorkType,
  workingDir?: string
): Promise<CreateSessionResponse> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/prompt-session`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ workType, workingDir }),
  })
  if (!res.ok) throw new Error(`Failed to create session: ${res.statusText}`)
  return res.json()
}

/** Fetch available skills from the backend. Returns empty array on failure. */
export async function fetchSkills(): Promise<SkillInfo[]> {
  try {
    const res = await fetch(`${API_BASE}/api/skills`)
    if (!res.ok) return []
    const data: ListSkillsResponse = await res.json()
    return data.skills
  } catch {
    return []
  }
}

export async function* sendPromptMessage(
  projectId: string,
  sessionId: string,
  content: string,
  enabledSkills?: string[],
): AsyncGenerator<string, void, unknown> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/prompt-session/${encodeURIComponent(sessionId)}/message`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ content, enabledSkills }),
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
  projectId: string,
  workingDir: string,
  content: string
): Promise<SavePromptResponse> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/prompt/save`, {
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
  projectId: string,
  request: SavePromptSessionRequest
): Promise<SavePromptSessionResponse> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/prompts`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  })
  if (!res.ok) throw new Error(`Failed to save prompt session: ${res.statusText}`)
  return res.json()
}

export async function listPrompts(
  projectId: string,
  params?: ListPromptsParams
): Promise<ListPromptsResponse> {
  const searchParams = new URLSearchParams()
  if (params?.projectName) searchParams.set('projectName', params.projectName)
  if (params?.search) searchParams.set('search', params.search)
  if (params?.limit) searchParams.set('limit', String(params.limit))
  if (params?.offset) searchParams.set('offset', String(params.offset))

  const url = `${API_BASE}/api/projects/${encodeURIComponent(projectId)}/prompts${searchParams.toString() ? `?${searchParams}` : ''}`
  const res = await fetch(url)
  if (!res.ok) throw new Error(`Failed to list prompts: ${res.statusText}`)
  return res.json()
}

export async function getPromptById(projectId: string, id: string): Promise<GetPromptResponse> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/prompts/${encodeURIComponent(id)}`)
  if (!res.ok) throw new Error(`Failed to get prompt: ${res.statusText}`)
  return res.json()
}

export async function deletePrompt(projectId: string, id: string): Promise<void> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/prompts/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  })
  if (!res.ok) throw new Error(`Failed to delete prompt: ${res.statusText}`)
}

// ============================================================================
// Prompt Inheritance API
// ============================================================================

export async function updatePromptParents(projectId: string, id: string, parentIds: string[]): Promise<void> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/prompts/${encodeURIComponent(id)}/parents`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(parentIds),
  })
  if (!res.ok) throw new Error(`Failed to update prompt parents: ${res.statusText}`)
}

export async function getResolvedPrompt(projectId: string, id: string): Promise<ResolvedPromptResponse> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/prompts/${encodeURIComponent(id)}/resolved`)
  if (!res.ok) throw new Error(`Failed to get resolved prompt: ${res.statusText}`)
  return res.json()
}
