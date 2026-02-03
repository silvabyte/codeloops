import { useMetrics } from '@/hooks/useMetrics'
import { useSessionEvents } from '@/hooks/useSSE'
import { SectionHeader } from '@/components/SectionHeader'
import { MetricCard } from '@/components/MetricCard'
import { DotChart } from '@/components/DotChart'
import { formatDuration } from '@/lib/utils'
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts'

const runInsightsTabs = [
  { label: 'Overview', path: '/run-insights' },
  { label: 'Status', path: '/run-insights/status' },
]

export function Stats() {
  const { metrics, loading, error, reload } = useMetrics()

  // Auto-refresh on SSE events
  useSessionEvents(() => {
    reload()
  })

  if (loading) {
    return (
      <div className="flex flex-col h-[calc(100vh-65px)]">
        <SectionHeader context="Run Insights" tabs={runInsightsTabs} />
        <div className="flex-1 overflow-auto">
          <div className="max-w-4xl mx-auto px-6 py-8">
            <div className="animate-pulse space-y-8">
              <div className="h-32 bg-muted rounded-lg" />
              <div className="h-32 bg-muted rounded-lg" />
              <div className="h-48 bg-muted rounded-lg" />
              <div className="h-32 bg-muted rounded-lg" />
            </div>
          </div>
        </div>
      </div>
    )
  }

  if (error || !metrics) {
    return (
      <div className="flex flex-col h-[calc(100vh-65px)]">
        <SectionHeader context="Run Insights" tabs={runInsightsTabs} />
        <div className="flex-1 overflow-auto">
          <div className="max-w-4xl mx-auto px-6 py-8">
            <div className="text-destructive">{error || 'Failed to load metrics'}</div>
          </div>
        </div>
      </div>
    )
  }

  // Compute waste breakdown
  const wasteCount = Math.round(metrics.totalSessions * metrics.wasteRate)

  return (
    <div className="flex flex-col h-[calc(100vh-65px)]">
      <SectionHeader context="Run Insights" tabs={runInsightsTabs} />

      <div className="flex-1 overflow-auto">
        <div className="max-w-4xl mx-auto px-6 py-8 space-y-8">
          {/* Session Efficacy */}
          <section>
            <h2 className="text-xs text-muted-foreground uppercase tracking-wider mb-4">
              Session Efficacy
            </h2>
            <div className="rounded-lg border border-border bg-card p-6">
              <div className="grid grid-cols-4 gap-6">
                <MetricCard
                  value={`${Math.round(metrics.successRate * 100)}%`}
                  label="Success Rate"
                />
                <MetricCard
                  value={`${Math.round(metrics.firstTrySuccessRate * 100)}%`}
                  label="First-Try Success"
                />
                <MetricCard
                  value={metrics.avgIterationsToSuccess.toFixed(1)}
                  label="Iterations to Success"
                />
                <MetricCard
                  value={formatDuration(metrics.avgCycleTimeSecs)}
                  label="Cycle Time"
                />
              </div>
              <div className="text-xs text-muted-foreground mt-4 text-center">
                {Math.round(metrics.wasteRate * 100)}% waste rate ({wasteCount} failed/interrupted sessions)
              </div>
            </div>
          </section>

          {/* Critic Performance */}
          <section>
            <h2 className="text-xs text-muted-foreground uppercase tracking-wider mb-4">
              Critic Performance
            </h2>
            <div className="rounded-lg border border-border bg-card p-6">
              <div className="grid grid-cols-3 gap-6">
                <MetricCard
                  value={`${Math.round(metrics.criticApprovalRate * 100)}%`}
                  label="Approval Rate"
                />
                <MetricCard
                  value={`${Math.round(metrics.avgFeedbackLength)} chars`}
                  label="Avg Feedback Length"
                />
                <MetricCard
                  value={`${Math.round(metrics.improvementRate * 100)}%`}
                  label="Improvement Rate"
                />
              </div>
              <div className="text-xs text-muted-foreground mt-4 text-center">
                (across {metrics.totalIterations} iterations)
              </div>
            </div>
          </section>

          {/* Activity Chart */}
          {metrics.sessionsOverTime.length > 0 && (
            <section>
              <h2 className="text-xs text-muted-foreground uppercase tracking-wider mb-4">
                Activity
              </h2>
              <div className="rounded-lg border border-border bg-card p-4">
                <ResponsiveContainer width="100%" height={160}>
                  <BarChart data={metrics.sessionsOverTime}>
                    <XAxis
                      dataKey="date"
                      tick={{ fontSize: 10, fill: '#a1a1aa' }}
                      axisLine={{ stroke: '#27272a' }}
                      tickLine={false}
                      tickFormatter={(value) => {
                        const d = new Date(value)
                        return `${d.getMonth() + 1}/${d.getDate()}`
                      }}
                    />
                    <YAxis
                      tick={{ fontSize: 10, fill: '#a1a1aa' }}
                      axisLine={{ stroke: '#27272a' }}
                      tickLine={false}
                      allowDecimals={false}
                      width={24}
                    />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: '#111118',
                        border: '1px solid #27272a',
                        borderRadius: '0.375rem',
                        fontSize: '0.75rem',
                      }}
                      labelFormatter={(value) => new Date(value).toLocaleDateString()}
                    />
                    <Bar dataKey="count" fill="#f59e0b" radius={[2, 2, 0, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            </section>
          )}

          {/* By Project */}
          {metrics.byProject.length > 0 && (
            <section>
              <h2 className="text-xs text-muted-foreground uppercase tracking-wider mb-4">
                By Project
              </h2>
              <div className="rounded-lg border border-border bg-card p-4">
                <DotChart projects={metrics.byProject} />
              </div>
            </section>
          )}
        </div>
      </div>
    </div>
  )
}
