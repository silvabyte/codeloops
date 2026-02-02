import type { AgenticMetrics, Session, SessionFilter, SessionStats, SessionSummary } from './types'

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3100'

export async function fetchSessions(filter?: SessionFilter): Promise<SessionSummary[]> {
  const params = new URLSearchParams()
  if (filter?.outcome) params.set('outcome', filter.outcome)
  if (filter?.after) params.set('after', filter.after)
  if (filter?.before) params.set('before', filter.before)
  if (filter?.search) params.set('search', filter.search)
  if (filter?.project) params.set('project', filter.project)

  const query = params.toString()
  const url = `${API_BASE}/api/sessions${query ? `?${query}` : ''}`
  const res = await fetch(url)
  if (!res.ok) throw new Error(`Failed to fetch sessions: ${res.statusText}`)
  return res.json()
}

export async function fetchSession(id: string): Promise<Session> {
  const res = await fetch(`${API_BASE}/api/sessions/${encodeURIComponent(id)}`)
  if (!res.ok) throw new Error(`Failed to fetch session: ${res.statusText}`)
  return res.json()
}

export async function fetchSessionDiff(id: string): Promise<string> {
  const res = await fetch(`${API_BASE}/api/sessions/${encodeURIComponent(id)}/diff`)
  if (!res.ok) throw new Error(`Failed to fetch diff: ${res.statusText}`)
  return res.text()
}

export async function fetchStats(): Promise<SessionStats> {
  const res = await fetch(`${API_BASE}/api/stats`)
  if (!res.ok) throw new Error(`Failed to fetch stats: ${res.statusText}`)
  return res.json()
}

export async function fetchMetrics(): Promise<AgenticMetrics> {
  const res = await fetch(`${API_BASE}/api/metrics`)
  if (!res.ok) throw new Error(`Failed to fetch metrics: ${res.statusText}`)
  return res.json()
}

export function getSSEUrl(): string {
  return `${API_BASE}/api/sessions/live`
}
