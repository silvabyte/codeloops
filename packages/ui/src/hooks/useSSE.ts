import { useEffect, useRef } from 'react'

/**
 * Hook that triggers a callback at regular intervals for auto-refresh.
 * This replaces the SSE-based approach since the /api/sessions/live endpoint
 * is not currently implemented.
 *
 * @param onRefresh Callback to trigger on each interval
 * @param intervalMs Refresh interval in milliseconds (default: 30000ms / 30s)
 */
export function useSessionEvents(onRefresh: () => void, intervalMs = 30000) {
  const onRefreshRef = useRef(onRefresh)

  useEffect(() => {
    onRefreshRef.current = onRefresh
  })

  useEffect(() => {
    // Skip polling if interval is 0 or negative
    if (intervalMs <= 0) return

    const interval = setInterval(() => {
      onRefreshRef.current()
    }, intervalMs)

    return () => clearInterval(interval)
  }, [intervalMs])
}
