import { useCallback, useState } from 'react'
import { StatsBar } from '@/components/StatsBar'
import { SessionFilters } from '@/components/SessionFilters'
import { SessionTable } from '@/components/SessionTable'
import { Welcome } from '@/components/Welcome'
import { useSessions } from '@/hooks/useSessions'
import { useStats } from '@/hooks/useStats'
import { useSessionEvents } from '@/hooks/useSSE'
import type { SessionFilter } from '@/api/types'

function DashboardSkeleton() {
  return (
    <div className="max-w-7xl mx-auto px-6 py-8 space-y-6">
      <div className="h-8 w-32 bg-muted rounded animate-pulse" />
      <div className="grid grid-cols-4 gap-4">
        {[1, 2, 3, 4].map((i) => (
          <div key={i} className="h-24 bg-muted rounded animate-pulse" />
        ))}
      </div>
      <div className="h-12 w-full bg-muted rounded animate-pulse" />
      <div className="h-64 w-full bg-muted rounded animate-pulse" />
    </div>
  )
}

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

  const hasSessions = sessions.length > 0
  const hasFilters = Object.values(filter).some(Boolean)

  // Show skeleton only on initial load
  if (loading && !hasSessions && !hasFilters) {
    return <DashboardSkeleton />
  }

  // Show welcome when no sessions and no filters applied
  if (!hasSessions && !hasFilters && !loading) {
    return (
      <div className="max-w-7xl mx-auto px-6 py-8">
        <Welcome />
      </div>
    )
  }

  return (
    <div className="max-w-7xl mx-auto px-6 py-8 space-y-6">
      <h1 className="text-2xl font-bold">Sessions</h1>

      <StatsBar stats={stats} loading={statsLoading} />

      <SessionFilters onFilterChange={handleFilterChange} />

      <SessionTable sessions={sessions} loading={loading} />
    </div>
  )
}
