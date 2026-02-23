import type { AgenticMetrics, Session, SessionFilter, SessionStats, SessionSummary } from './types'
import type { ProjectListResponse, ProjectRecord } from '@/types/project'

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3100'

// ============================================================================
// Project API
// ============================================================================

export async function fetchProjects(): Promise<ProjectListResponse> {
  const res = await fetch(`${API_BASE}/api/projects`)
  if (!res.ok) throw new Error(`Failed to fetch projects: ${res.statusText}`)
  return res.json()
}

export async function fetchProject(projectId: string): Promise<ProjectRecord> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}`)
  if (!res.ok) throw new Error(`Failed to fetch project: ${res.statusText}`)
  return res.json()
}

export async function createProject(path: string, name?: string): Promise<ProjectRecord> {
  const res = await fetch(`${API_BASE}/api/projects`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, name }),
  })
  if (res.status === 409) throw new Error('A project already exists for this directory')
  if (res.status === 400) {
    const text = await res.text()
    throw new Error(text || 'Directory does not exist')
  }
  if (!res.ok) throw new Error(`Failed to create project: ${res.statusText}`)
  return res.json()
}

export async function deleteProject(projectId: string): Promise<void> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}`, {
    method: 'DELETE',
  })
  if (!res.ok) throw new Error(`Failed to delete project: ${res.statusText}`)
}

// ============================================================================
// Project-scoped Session API
// ============================================================================

export async function fetchSessions(projectId: string, filter?: SessionFilter): Promise<SessionSummary[]> {
  const params = new URLSearchParams()
  if (filter?.outcome) params.set('outcome', filter.outcome)
  if (filter?.after) params.set('after', filter.after)
  if (filter?.before) params.set('before', filter.before)
  if (filter?.search) params.set('search', filter.search)
  if (filter?.project) params.set('project', filter.project)

  const query = params.toString()
  const url = `${API_BASE}/api/projects/${encodeURIComponent(projectId)}/sessions${query ? `?${query}` : ''}`
  const res = await fetch(url)
  if (!res.ok) throw new Error(`Failed to fetch sessions: ${res.statusText}`)
  return res.json()
}

export async function fetchSession(projectId: string, id: string): Promise<Session> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/sessions/${encodeURIComponent(id)}`)
  if (!res.ok) throw new Error(`Failed to fetch session: ${res.statusText}`)
  return res.json()
}

export async function fetchSessionDiff(projectId: string, id: string): Promise<string> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/sessions/${encodeURIComponent(id)}/diff`)
  if (!res.ok) throw new Error(`Failed to fetch diff: ${res.statusText}`)
  return res.text()
}

export async function fetchStats(projectId: string): Promise<SessionStats> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/stats`)
  if (!res.ok) throw new Error(`Failed to fetch stats: ${res.statusText}`)
  return res.json()
}

export async function fetchMetrics(projectId: string): Promise<AgenticMetrics> {
  const res = await fetch(`${API_BASE}/api/projects/${encodeURIComponent(projectId)}/metrics`)
  if (!res.ok) throw new Error(`Failed to fetch metrics: ${res.statusText}`)
  return res.json()
}
