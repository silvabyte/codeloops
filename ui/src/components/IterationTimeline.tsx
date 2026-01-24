import { useState } from 'react'
import { cn, formatDuration } from '@/lib/utils'
import type { Iteration } from '@/api/types'

interface IterationTimelineProps {
  iterations: Iteration[]
}

function decisionColor(decision: string): string {
  switch (decision.toUpperCase()) {
    case 'DONE': return 'bg-success'
    case 'CONTINUE': return 'bg-warning'
    default: return 'bg-destructive'
  }
}

function decisionTextColor(decision: string): string {
  switch (decision.toUpperCase()) {
    case 'DONE': return 'text-success'
    case 'CONTINUE': return 'text-warning'
    default: return 'text-destructive'
  }
}

export function IterationTimeline({ iterations }: IterationTimelineProps) {
  const [expandedId, setExpandedId] = useState<number | null>(null)

  if (iterations.length === 0) {
    return <div className="text-muted-foreground text-sm">No iterations yet.</div>
  }

  const maxDuration = Math.max(...iterations.map(i => i.actor_duration_secs))

  return (
    <div className="space-y-2">
      <div className="text-xs text-muted-foreground uppercase tracking-wider mb-3">
        Iteration Timeline
      </div>

      {/* Timeline bars */}
      <div className="space-y-1.5">
        {iterations.map((iter) => {
          const widthPct = maxDuration > 0 ? (iter.actor_duration_secs / maxDuration) * 100 : 100
          const isExpanded = expandedId === iter.iteration_number

          return (
            <div key={iter.iteration_number}>
              <div
                className="flex items-center gap-3 cursor-pointer group"
                onClick={() => setExpandedId(isExpanded ? null : iter.iteration_number)}
              >
                <span className="text-xs text-muted-foreground w-4 text-right">
                  {iter.iteration_number}
                </span>
                <div className="flex-1 h-6 bg-secondary rounded overflow-hidden">
                  <div
                    className={cn('h-full rounded transition-all', decisionColor(iter.critic_decision), 'opacity-60 group-hover:opacity-80')}
                    style={{ width: `${Math.max(widthPct, 5)}%` }}
                  />
                </div>
                <span className="text-xs text-muted-foreground w-12 text-right">
                  {formatDuration(iter.actor_duration_secs)}
                </span>
                <span className={cn('text-xs font-medium w-16', decisionTextColor(iter.critic_decision))}>
                  {iter.critic_decision}
                </span>
              </div>

              {isExpanded && (
                <div className="ml-7 mt-2 mb-3 p-3 rounded-lg border border-border bg-card text-sm space-y-2">
                  <div className="flex gap-4 text-xs text-muted-foreground">
                    <span>Exit: {iter.actor_exit_code}</span>
                    <span>{iter.git_files_changed} files changed</span>
                    <span>{new Date(iter.timestamp).toLocaleTimeString()}</span>
                  </div>
                  {iter.feedback && (
                    <div>
                      <div className="text-xs text-muted-foreground mb-1">Critic Feedback:</div>
                      <div className="text-xs whitespace-pre-wrap bg-secondary/50 p-2 rounded">
                        {iter.feedback}
                      </div>
                    </div>
                  )}
                  {iter.git_diff && (
                    <details>
                      <summary className="text-xs text-muted-foreground cursor-pointer hover:text-foreground">
                        View diff ({iter.git_files_changed} files)
                      </summary>
                      <pre className="text-xs mt-2 p-2 bg-secondary/50 rounded overflow-x-auto max-h-64 overflow-y-auto">
                        {iter.git_diff}
                      </pre>
                    </details>
                  )}
                </div>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
