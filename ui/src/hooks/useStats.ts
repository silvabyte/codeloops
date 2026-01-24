import { useCallback, useEffect, useState } from 'react'
import { fetchStats } from '@/api/client'
import type { SessionStats } from '@/api/types'

export function useStats() {
  const [stats, setStats] = useState<SessionStats | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setLoading(true)
      const data = await fetchStats()
      setStats(data)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load stats')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { load() }, [load])

  return { stats, loading, error, reload: load }
}
