import { useEffect, useRef } from 'react'
import { getSSEUrl } from '@/api/client'
import type { SessionEvent } from '@/api/types'

export function useSessionEvents(onEvent: (event: SessionEvent) => void) {
  const onEventRef = useRef(onEvent)

  useEffect(() => {
    onEventRef.current = onEvent
  })

  useEffect(() => {
    const source = new EventSource(getSSEUrl())

    const handler = (e: MessageEvent) => {
      try {
        onEventRef.current(JSON.parse(e.data))
      } catch {
        // ignore parse errors
      }
    }

    source.addEventListener('session_created', handler)
    source.addEventListener('session_updated', handler)
    source.addEventListener('session_completed', handler)

    return () => source.close()
  }, [])
}
