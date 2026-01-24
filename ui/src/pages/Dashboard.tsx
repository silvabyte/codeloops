import { useCallback, useState } from 'react'
import { StatsBar } from '@/components/StatsBar'
import { SessionFilters } from '@/components/SessionFilters'
import { SessionTable } from '@/components/SessionTable'
import { useSessions } from '@/hooks/useSessions'
import { useStats } from '@/hooks/useStats'
import { useSessionEvents } from '@/hooks/useSSE'
import type { SessionFilter } from '@/api/types'

export function Dashboard() {
  const [filter, setFilter] = useState<SessionFilter>({})
  const { sessions, loading, reload } = useSessions(filter)
  const { stats, loading: statsLoading, reload: reloadStats } = useStats()

  const handleFilterChange = useCallback((f: SessionFilter) => {
    setFilter(f)
  }, [])

  // Auto-refresh on SSE events
  useSessionEvents(() => {
    reload()
    reloadStats()
  })

  return (
    <div className="max-w-7xl mx-auto px-6 py-8 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Sessions</h1>
        <a
          href="/stats"
          className="text-sm text-primary hover:text-primary/80 transition-colors"
        >
          View Stats
        </a>
      </div>

      <StatsBar stats={stats} loading={statsLoading} />

      <SessionFilters onFilterChange={handleFilterChange} />

      <SessionTable sessions={sessions} loading={loading} />
    </div>
  )
}
