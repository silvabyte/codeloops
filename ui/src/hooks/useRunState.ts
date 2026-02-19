import { useCallback, useEffect, useRef, useState } from 'react'
import { fetchSession } from '@/api/client'
import type { Iteration, Session } from '@/api/types'

export interface RunState {
  iterations: Iteration[]
  currentIteration: number
  isLive: boolean
  isComplete: boolean
  outcome: string | null
  session: Session | null
  loading: boolean
  error: string | null
}

/**
 * Polls GET /api/sessions/{id} at a configurable interval to track
 * iteration phase state for live run visualization.
 *
 * Stops polling when the session has an outcome (completed/failed/etc).
 */
export function useRunState(sessionId: string | undefined, pollInterval = 3000): RunState {
  const [session, setSession] = useState<Session | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const fetchData = useCallback(async () => {
    if (!sessionId) return
    try {
      const data = await fetchSession(sessionId)
      setSession(data)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to fetch session')
    } finally {
      setLoading(false)
    }
  }, [sessionId])

  // Initial fetch
  useEffect(() => {
    setLoading(true)
    fetchData()
  }, [fetchData])

  // Polling
  useEffect(() => {
    if (!sessionId || pollInterval <= 0) return

    // Don't poll if session is complete
    if (session?.outcome) return

    intervalRef.current = setInterval(fetchData, pollInterval)

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current)
        intervalRef.current = null
      }
    }
  }, [sessionId, pollInterval, fetchData, session?.outcome])

  const iterations = session?.iterations ?? []
  const isComplete = session?.outcome != null
  const isLive = !isComplete && session != null
  const currentIteration = iterations.length > 0 ? iterations[iterations.length - 1].iterationNumber : 0

  return {
    iterations,
    currentIteration,
    isLive,
    isComplete,
    outcome: session?.outcome ?? null,
    session,
    loading,
    error,
  }
}
