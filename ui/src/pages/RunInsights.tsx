import { useCallback, useState } from 'react'
import { HealthPulse } from '@/components/HealthPulse'
import { SessionFilters } from '@/components/SessionFilters'
import { SessionList } from '@/components/SessionList'
import { SectionHeader } from '@/components/SectionHeader'
import { Welcome } from '@/components/Welcome'
import { useSessions } from '@/hooks/useSessions'
import { useStats } from '@/hooks/useStats'
import { useSessionEvents } from '@/hooks/useSSE'
import type { SessionFilter } from '@/api/types'

const runInsightsTabs = [
  { label: 'Overview', path: '/run-insights' },
  { label: 'Status', path: '/run-insights/status' },
]

function RunInsightsSkeleton() {
  return (
    <div className="max-w-4xl mx-auto px-6 py-8 space-y-8">
      <div className="h-8 w-64 bg-muted rounded animate-pulse" />
      <div className="h-10 w-full bg-muted rounded animate-pulse" />
      <div className="space-y-3">
        {[1, 2, 3, 4, 5].map((i) => (
          <div key={i} className="h-20 bg-muted rounded-lg animate-pulse" />
        ))}
      </div>
    </div>
  )
}

export function RunInsights() {
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
    return <RunInsightsSkeleton />
  }

  // Show welcome when no sessions and no filters applied
  if (!hasSessions && !hasFilters && !loading) {
    return (
      <div className="max-w-4xl mx-auto px-6 py-8">
        <Welcome />
      </div>
    )
  }

  return (
    <div className="flex flex-col h-[calc(100vh-65px)]">
      <SectionHeader context="Run Insights" tabs={runInsightsTabs} />

      <div className="flex-1 overflow-auto">
        <div className="max-w-4xl mx-auto px-6 py-8 space-y-8">
          <HealthPulse stats={stats} loading={statsLoading} />
          <SessionFilters onFilterChange={handleFilterChange} />
          <SessionList sessions={sessions} loading={loading} />
        </div>
      </div>
    </div>
  )
}
