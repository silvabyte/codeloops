import { useCallback, useEffect, useState } from 'react'
import { fetchSession, fetchSessionDiff } from '@/api/client'
import type { Session } from '@/api/types'

export function useSession(id: string | undefined) {
  const [session, setSession] = useState<Session | null>(null)
  const [diff, setDiff] = useState<string>('')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    if (!id) return
    try {
      setLoading(true)
      const [sessionData, diffData] = await Promise.all([
        fetchSession(id),
        fetchSessionDiff(id),
      ])
      setSession(sessionData)
      setDiff(diffData)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load session')
    } finally {
      setLoading(false)
    }
  }, [id])

  useEffect(() => { load() }, [load])

  return { session, diff, loading, error, reload: load }
}
