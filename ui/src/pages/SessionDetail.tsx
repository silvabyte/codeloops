import { useParams, useNavigate } from 'react-router-dom'
import { useSession } from '@/hooks/useSession'
import { IterationConversation } from '@/components/IterationConversation'
import { ContentBlock } from '@/components/ContentBlock'
import { CopyButton } from '@/components/CopyButton'
import { formatDuration } from '@/lib/utils'
import { useState } from 'react'
import { cn } from '@/lib/utils'

type Tab = 'prompt' | 'iterations' | 'summary' | 'diff'

export function SessionDetail() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { session, diff, loading, error } = useSession(id)
  const [activeTab, setActiveTab] = useState<Tab>('prompt')

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
    success: 'bg-success',
    failed: 'bg-destructive',
    active: 'bg-cyan',
    interrupted: 'bg-amber',
    max_iterations_reached: 'bg-amber',
  }
  const outcomeTextColor: Record<string, string> = {
    success: 'text-success',
    failed: 'text-destructive',
    active: 'text-cyan',
    interrupted: 'text-amber',
    max_iterations_reached: 'text-amber',
  }

  const tabs: { key: Tab; label: string }[] = [
    { key: 'prompt', label: 'Prompt' },
    { key: 'iterations', label: 'Iterations' },
    { key: 'summary', label: 'Summary' },
    { key: 'diff', label: 'Diff' },
  ]

  return (
    <div className="max-w-5xl mx-auto px-6 py-8 space-y-6">
      {/* Minimal Header */}
      <div>
        <button
          onClick={() => navigate('/run-insights')}
          className="text-sm text-muted-foreground hover:text-foreground mb-4 block"
        >
          &larr; Back to Run Insights
        </button>

        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-xl font-bold">Session</h1>
            <div className="text-sm text-muted-foreground mt-1 font-mono">{session.id}</div>
          </div>

          {/* Right side metrics */}
          <div className="flex items-center gap-4 text-sm">
            {/* Status */}
            <div className="flex items-center gap-2">
              <span
                className={cn(
                  'w-2 h-2 rounded-full',
                  outcomeColor[outcomeLabel] || 'bg-muted-foreground'
                )}
              />
              <span className={cn('font-medium', outcomeTextColor[outcomeLabel] || 'text-muted-foreground')}>
                {outcomeLabel}
              </span>
            </div>

            {/* Duration */}
            {session.end && (
              <span className="text-muted-foreground">
                {formatDuration(session.end.duration_secs)}
              </span>
            )}

            {/* Iteration count */}
            <span className="text-muted-foreground">
              {session.iterations.length} iteration{session.iterations.length !== 1 ? 's' : ''}
            </span>
          </div>
        </div>
      </div>

      {/* Tab Navigation */}
      <div className="border-b border-border">
        <div className="flex gap-6">
          {tabs.map((tab) => (
            <button
              key={tab.key}
              onClick={() => setActiveTab(tab.key)}
              className={cn(
                'pb-2 text-sm transition-colors border-b-2',
                activeTab === tab.key
                  ? 'border-amber text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              )}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Tab Content */}
      <div className="min-h-[400px]">
        {activeTab === 'prompt' && (
          <ContentBlock
            label="Prompt"
            content={session.start.prompt}
          />
        )}

        {activeTab === 'iterations' && (
          <IterationConversation iterations={session.iterations} />
        )}

        {activeTab === 'summary' && (
          <div>
            {session.end?.summary ? (
              <div className="space-y-4">
                <ContentBlock
                  label="Summary"
                  content={session.end.summary}
                />
                {session.end.confidence != null && (
                  <div className="text-sm text-muted-foreground">
                    Confidence: {Math.round(session.end.confidence * 100)}%
                  </div>
                )}
              </div>
            ) : (
              <div className="text-muted-foreground text-sm py-8 text-center">
                {session.end ? 'No summary available.' : 'Session in progress...'}
              </div>
            )}
          </div>
        )}

        {activeTab === 'diff' && (
          <div>
            {diff ? (
              <div className="rounded-lg border border-border overflow-hidden">
                <div className="flex items-center justify-between px-4 py-2 bg-elevated/30 border-b border-border">
                  <span className="text-xs text-muted-foreground uppercase tracking-wider font-medium">
                    Cumulative Diff
                  </span>
                  <CopyButton content={diff} />
                </div>
                <pre className="p-4 text-xs overflow-x-auto max-h-[600px] overflow-y-auto bg-surface font-mono">
                  {diff.split('\n').map((line, i) => {
                    let className = ''
                    if (line.startsWith('+') && !line.startsWith('+++')) {
                      className = 'text-success bg-success/10'
                    } else if (line.startsWith('-') && !line.startsWith('---')) {
                      className = 'text-destructive bg-destructive/10'
                    } else if (line.startsWith('@@')) {
                      className = 'text-cyan'
                    } else if (line.startsWith('diff ') || line.startsWith('index ')) {
                      className = 'text-muted-foreground font-bold'
                    }
                    return (
                      <div key={i} className={className}>
                        {line}
                      </div>
                    )
                  })}
                </pre>
              </div>
            ) : (
              <div className="text-muted-foreground text-sm py-8 text-center">
                No diffs available.
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
