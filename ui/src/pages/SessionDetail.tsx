import { useParams, useNavigate } from 'react-router-dom'
import { useSession } from '@/hooks/useSession'
import { IterationTimeline } from '@/components/IterationTimeline'
import { CriticTrail } from '@/components/CriticTrail'
import { DiffViewer } from '@/components/DiffViewer'
import { formatDuration } from '@/lib/utils'
import { useState } from 'react'
import { cn } from '@/lib/utils'

type Tab = 'timeline' | 'feedback' | 'diff'

export function SessionDetail() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { session, diff, loading, error } = useSession(id)
  const [activeTab, setActiveTab] = useState<Tab>('timeline')

  if (loading) {
    return (
      <div className="max-w-5xl mx-auto px-6 py-8">
        <div className="animate-pulse space-y-4">
          <div className="h-6 bg-muted rounded w-48" />
          <div className="h-4 bg-muted rounded w-96" />
          <div className="h-64 bg-muted rounded" />
        </div>
      </div>
    )
  }

  if (error || !session) {
    return (
      <div className="max-w-5xl mx-auto px-6 py-8">
        <div className="text-destructive">{error || 'Session not found'}</div>
        <button
          onClick={() => navigate('/run-insights')}
          className="mt-4 text-sm text-primary hover:underline"
        >
          Back to Run Insights
        </button>
      </div>
    )
  }

  const outcomeLabel = session.end?.outcome || 'active'
  const outcomeColor: Record<string, string> = {
    success: 'text-success',
    failed: 'text-destructive',
    active: 'text-primary',
    interrupted: 'text-warning',
    max_iterations_reached: 'text-warning',
  }

  return (
    <div className="max-w-5xl mx-auto px-6 py-8 space-y-6">
      {/* Header */}
      <div>
        <button
          onClick={() => navigate('/run-insights')}
          className="text-sm text-muted-foreground hover:text-foreground mb-3 block"
        >
          &larr; Back to Run Insights
        </button>
        <div className="flex items-center gap-3">
          <h1 className="text-xl font-bold">Session</h1>
          <span className={cn('text-sm font-medium', outcomeColor[outcomeLabel] || 'text-muted-foreground')}>
            {outcomeLabel}
          </span>
        </div>
        <div className="text-sm text-muted-foreground mt-1">{session.id}</div>
      </div>

      {/* Meta */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
        <div>
          <div className="text-muted-foreground text-xs">Started</div>
          <div>{new Date(session.start.timestamp).toLocaleString()}</div>
        </div>
        <div>
          <div className="text-muted-foreground text-xs">Duration</div>
          <div>{session.end ? formatDuration(session.end.duration_secs) : '...'}</div>
        </div>
        <div>
          <div className="text-muted-foreground text-xs">Actor</div>
          <div>{session.start.actor_agent}{session.start.actor_model ? ` (${session.start.actor_model})` : ''}</div>
        </div>
        <div>
          <div className="text-muted-foreground text-xs">Critic</div>
          <div>{session.start.critic_agent}{session.start.critic_model ? ` (${session.start.critic_model})` : ''}</div>
        </div>
      </div>

      {/* Prompt */}
      <div className="rounded-lg border border-border bg-card p-4">
        <div className="text-xs text-muted-foreground uppercase tracking-wider mb-2">Prompt</div>
        <div className="text-sm whitespace-pre-wrap">{session.start.prompt}</div>
      </div>

      {/* Summary */}
      {session.end?.summary && (
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="text-xs text-muted-foreground uppercase tracking-wider mb-2">Summary</div>
          <div className="text-sm">{session.end.summary}</div>
          {session.end.confidence != null && (
            <div className="text-xs text-muted-foreground mt-2">
              Confidence: {Math.round(session.end.confidence * 100)}%
            </div>
          )}
        </div>
      )}

      {/* Tabs */}
      <div className="border-b border-border">
        <div className="flex gap-6">
          {(['timeline', 'feedback', 'diff'] as Tab[]).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={cn(
                'pb-2 text-sm capitalize transition-colors border-b-2',
                activeTab === tab
                  ? 'border-primary text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              )}
            >
              {tab}
            </button>
          ))}
        </div>
      </div>

      {/* Tab content */}
      {activeTab === 'timeline' && <IterationTimeline iterations={session.iterations} />}
      {activeTab === 'feedback' && <CriticTrail iterations={session.iterations} />}
      {activeTab === 'diff' && <DiffViewer diff={diff} />}
    </div>
  )
}
