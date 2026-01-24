import { useCallback, useEffect, useState } from 'react'
import { fetchSessions } from '@/api/client'
import type { SessionFilter, SessionSummary } from '@/api/types'

export function useSessions(filter?: SessionFilter) {
  const [sessions, setSessions] = useState<SessionSummary[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setLoading(true)
      const data = await fetchSessions(filter)
      setSessions(data)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load sessions')
    } finally {
      setLoading(false)
    }
  }, [filter?.outcome, filter?.after, filter?.before, filter?.search, filter?.project])

  useEffect(() => { load() }, [load])

  return { sessions, loading, error, reload: load }
}
