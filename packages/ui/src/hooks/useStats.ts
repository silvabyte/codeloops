import { useCallback, useEffect, useState } from 'react'
import { fetchStats } from '@/api/client'
import { useCurrentProject } from '@/hooks/useProject'
import type { SessionStats } from '@/api/types'

export function useStats() {
  const projectId = useCurrentProject()
  const [stats, setStats] = useState<SessionStats | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setLoading(true)
      const data = await fetchStats(projectId)
      setStats(data)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load stats')
    } finally {
      setLoading(false)
    }
  }, [projectId])

  useEffect(() => { load() }, [load])

  return { stats, loading, error, reload: load }
}
