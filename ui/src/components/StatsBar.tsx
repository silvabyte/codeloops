import { cn, formatDuration } from '@/lib/utils'
import type { SessionStats } from '@/api/types'

interface StatsBarProps {
  stats: SessionStats | null
  loading: boolean
}

function StatCard({ label, value, className }: { label: string; value: string; className?: string }) {
  return (
    <div className={cn("rounded-lg border border-border bg-card p-4", className)}>
      <div className="text-xs text-muted-foreground uppercase tracking-wider">{label}</div>
      <div className="text-2xl font-bold mt-1">{value}</div>
    </div>
  )
}

export function StatsBar({ stats, loading }: StatsBarProps) {
  if (loading || !stats) {
    return (
      <div className="grid grid-cols-4 gap-4">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="rounded-lg border border-border bg-card p-4 animate-pulse">
            <div className="h-3 bg-muted rounded w-20 mb-2" />
            <div className="h-7 bg-muted rounded w-16" />
          </div>
        ))}
      </div>
    )
  }

  const successColor = stats.success_rate >= 0.7 ? 'text-success' : stats.success_rate >= 0.4 ? 'text-warning' : 'text-destructive'

  return (
    <div className="grid grid-cols-4 gap-4">
      <StatCard label="Total Sessions" value={String(stats.total_sessions)} />
      <StatCard
        label="Success Rate"
        value={`${Math.round(stats.success_rate * 100)}%`}
        className={successColor}
      />
      <StatCard label="Avg Iterations" value={stats.avg_iterations.toFixed(1)} />
      <StatCard label="Avg Duration" value={formatDuration(stats.avg_duration_secs)} />
    </div>
  )
}
