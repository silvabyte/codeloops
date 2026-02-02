import { useState, useEffect, useCallback } from 'react'
import { fetchMetrics } from '@/api/client'
import type { AgenticMetrics } from '@/api/types'

export function useMetrics() {
  const [metrics, setMetrics] = useState<AgenticMetrics | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setLoading(true)
      const data = await fetchMetrics()
      setMetrics(data)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load metrics')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  return { metrics, loading, error, reload: load }
}
