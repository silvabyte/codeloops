import { useCallback, useEffect, useState } from 'react'
import { fetchSessions } from '@/api/client'
import type { SessionFilter, SessionSummary } from '@/api/types'

export function useSessions(filter?: SessionFilter) {
  const [sessions, setSessions] = useState<SessionSummary[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const outcome = filter?.outcome
  const after = filter?.after
  const before = filter?.before
  const search = filter?.search
  const project = filter?.project

  const load = useCallback(async () => {
    try {
      setLoading(true)
      const data = await fetchSessions({ outcome, after, before, search, project })
      setSessions(data)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load sessions')
    } finally {
      setLoading(false)
    }
  }, [outcome, after, before, search, project])

  useEffect(() => { load() }, [load])

  return { sessions, loading, error, reload: load }
}
