import { useStats } from '@/hooks/useStats'
import { SectionHeader } from '@/components/SectionHeader'
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts'

const runInsightsTabs = [
  { label: 'Overview', path: '/run-insights' },
  { label: 'Status', path: '/run-insights/status' },
]

export function Stats() {
  const { stats, loading, error } = useStats()

  if (loading) {
    return (
      <div className="max-w-5xl mx-auto px-6 py-8">
        <div className="animate-pulse space-y-4">
          <div className="h-6 bg-muted rounded w-48" />
          <div className="h-64 bg-muted rounded" />
        </div>
      </div>
    )
  }

  if (error || !stats) {
    return (
      <div className="max-w-5xl mx-auto px-6 py-8">
        <div className="text-destructive">{error || 'Failed to load stats'}</div>
      </div>
    )
  }

  return (
    <div className="flex flex-col h-[calc(100vh-65px)]">
      <SectionHeader context="Run Insights" tabs={runInsightsTabs} />

      <div className="flex-1 overflow-auto">
        <div className="max-w-5xl mx-auto px-6 py-6 space-y-6">
          {/* Summary cards */}
      <div className="grid grid-cols-4 gap-4">
        <SummaryCard label="Total Sessions" value={String(stats.total_sessions)} />
        <SummaryCard label="Success Rate" value={`${Math.round(stats.success_rate * 100)}%`} />
        <SummaryCard label="Avg Iterations" value={stats.avg_iterations.toFixed(1)} />
        <SummaryCard label="Avg Duration" value={`${Math.round(stats.avg_duration_secs / 60)}m`} />
      </div>

      {/* Sessions over time chart */}
      {stats.sessions_over_time.length > 0 && (
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="text-xs text-muted-foreground uppercase tracking-wider mb-4">
            Sessions Over Time
          </div>
          <ResponsiveContainer width="100%" height={200}>
            <BarChart data={stats.sessions_over_time}>
              <XAxis
                dataKey="date"
                tick={{ fontSize: 11, fill: '#a1a1aa' }}
                axisLine={{ stroke: '#27272a' }}
                tickLine={false}
              />
              <YAxis
                tick={{ fontSize: 11, fill: '#a1a1aa' }}
                axisLine={{ stroke: '#27272a' }}
                tickLine={false}
                allowDecimals={false}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: '#111118',
                  border: '1px solid #27272a',
                  borderRadius: '0.375rem',
                  fontSize: '0.75rem',
                }}
              />
              <Bar dataKey="count" fill="#6366f1" radius={[4, 4, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* By project */}
      {stats.by_project.length > 0 && (
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="text-xs text-muted-foreground uppercase tracking-wider mb-4">
            By Project
          </div>
          <div className="space-y-3">
            {stats.by_project.map((p) => (
              <div key={p.project} className="flex items-center gap-4">
                <span className="text-sm w-40 truncate">{p.project}</span>
                <div className="flex-1 h-4 bg-secondary rounded overflow-hidden">
                  <div
                    className="h-full bg-primary/60 rounded"
                    style={{ width: `${(p.total / stats.total_sessions) * 100}%` }}
                  />
                </div>
                <span className="text-xs text-muted-foreground w-20 text-right">
                  {p.total} ({Math.round(p.success_rate * 100)}%)
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
        </div>
      </div>
    </div>
  )
}

function SummaryCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="text-xs text-muted-foreground uppercase tracking-wider">{label}</div>
      <div className="text-2xl font-bold mt-1">{value}</div>
    </div>
  )
}
