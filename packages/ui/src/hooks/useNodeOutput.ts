import { useEffect, useRef, useState } from 'react'
import { useCurrentProject } from '@/hooks/useProject'

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3100'

export interface OutputLine {
  line: string
  stream: 'stdout' | 'stderr'
}

/**
 * Connects to the SSE endpoint for live output streaming from an in-progress node.
 * For completed nodes, output is served directly from the session data (no SSE needed).
 *
 * Returns accumulated output lines and streaming status.
 */
export function useNodeOutput(
  sessionId: string,
  iterationNumber: number,
  phase: 'actor' | 'critic',
  enabled: boolean,
): { lines: OutputLine[]; isStreaming: boolean } {
  const projectId = useCurrentProject()
  const [lines, setLines] = useState<OutputLine[]>([])
  const [isStreaming, setIsStreaming] = useState(false)
  const eventSourceRef = useRef<EventSource | null>(null)

  useEffect(() => {
    if (!enabled) {
      // Clean up if disabled
      if (eventSourceRef.current) {
        eventSourceRef.current.close()
        eventSourceRef.current = null
      }
      return
    }

    // Reset state for new connection
    setLines([])
    setIsStreaming(true)

    const url = `${API_BASE}/api/projects/${encodeURIComponent(projectId)}/sessions/${encodeURIComponent(sessionId)}/output/${iterationNumber}/${phase}`
    const es = new EventSource(url)
    eventSourceRef.current = es

    es.onmessage = (event) => {
      if (event.data === '[DONE]') {
        setIsStreaming(false)
        es.close()
        eventSourceRef.current = null
        return
      }

      try {
        const parsed = JSON.parse(event.data) as { line: string; stream: string }
        setLines((prev) => [
          ...prev,
          { line: parsed.line, stream: parsed.stream as 'stdout' | 'stderr' },
        ])
      } catch {
        // Skip unparseable events (keep-alive, etc.)
      }
    }

    es.onerror = () => {
      setIsStreaming(false)
      es.close()
      eventSourceRef.current = null
    }

    return () => {
      es.close()
      eventSourceRef.current = null
    }
  }, [projectId, sessionId, iterationNumber, phase, enabled])

  return { lines, isStreaming }
}
