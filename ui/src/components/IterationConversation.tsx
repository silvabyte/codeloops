import { useState } from 'react'
import { cn, formatDuration } from '@/lib/utils'
import type { Iteration } from '@/api/types'
import { ContentBlock } from './ContentBlock'
import { CopyButton } from './CopyButton'

/**
 * IterationConversation - Conversation-style view of actor-critic iterations.
 * Displays each iteration as a card with actor output, critic feedback, and expandable diff.
 *
 * Design: Each iteration shows the dialogue between actor and critic,
 * making it easy to understand the decision-making process.
 */
interface IterationConversationProps {
  iterations: Iteration[]
}

function DecisionBadge({ decision }: { decision: string }) {
  const upper = decision.toUpperCase()
  return (
    <span
      className={cn(
        'text-xs font-medium px-2 py-0.5 rounded',
        upper === 'DONE' && 'text-success bg-success/10',
        upper === 'CONTINUE' && 'text-amber bg-amber/10',
        !['DONE', 'CONTINUE'].includes(upper) && 'text-destructive bg-destructive/10'
      )}
    >
      {upper}
    </span>
  )
}

function DiffPreview({ diff, filesChanged }: { diff: string; filesChanged: number }) {
  const [expanded, setExpanded] = useState(false)

  if (!diff) return null

  const lines = diff.split('\n')

  return (
    <div>
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
      >
        <span className="text-xs">{expanded ? '▾' : '▸'}</span>
        <span>View diff ({filesChanged} files changed)</span>
      </button>

      {expanded && (
        <div className="mt-3 rounded-lg border border-border overflow-hidden">
          <div className="flex items-center justify-between px-3 py-1.5 bg-elevated/30 border-b border-border">
            <span className="text-xs text-muted-foreground">Diff</span>
            <CopyButton content={diff} />
          </div>
          <pre className="p-3 text-xs overflow-x-auto max-h-64 overflow-y-auto bg-surface">
            {lines.map((line, i) => {
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
      )}
    </div>
  )
}

export function IterationConversation({ iterations }: IterationConversationProps) {
  if (iterations.length === 0) {
    return (
      <div className="text-muted-foreground text-sm py-8 text-center">
        No iterations yet.
      </div>
    )
  }

  const finalIteration = iterations[iterations.length - 1]

  return (
    <div className="space-y-6">
      {iterations.map((iter) => {
        const isFinal = iter.iteration_number === finalIteration.iteration_number

        return (
          <div
            key={iter.iteration_number}
            className="rounded-lg border border-border bg-card overflow-hidden"
          >
            {/* Iteration Header */}
            <div className="flex items-center justify-between px-4 py-3 border-b border-border bg-elevated/20">
              <div className="flex items-center gap-2">
                {isFinal && <span title="Final iteration">★</span>}
                <span className="text-sm font-medium">
                  ITERATION {iter.iteration_number}
                </span>
              </div>
              <span className="text-xs text-muted-foreground">
                {formatDuration(iter.actor_duration_secs)}
              </span>
            </div>

            {/* Iteration Content */}
            <div className="p-4 space-y-4">
              {/* Actor Output */}
              <ContentBlock
                label="Actor"
                content={iter.actor_output || '(no output)'}
                variant="actor"
              />

              {/* Critic Block */}
              <div
                className={cn(
                  'bg-surface rounded-lg border border-border overflow-hidden',
                  'border-l-2 border-l-amber-dim'
                )}
              >
                <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-elevated/30">
                  <div className="flex items-center gap-3">
                    <span className="text-xs text-muted-foreground uppercase tracking-wider font-medium">
                      Critic
                    </span>
                    <DecisionBadge decision={iter.critic_decision} />
                  </div>
                  {iter.feedback && <CopyButton content={iter.feedback} />}
                </div>
                <div className="p-4">
                  {iter.feedback ? (
                    <pre className="text-sm whitespace-pre-wrap font-mono leading-relaxed">
                      {iter.feedback}
                    </pre>
                  ) : (
                    <span className="text-sm text-muted-foreground italic">
                      No feedback provided
                    </span>
                  )}
                </div>
              </div>

              {/* Expandable Diff */}
              <DiffPreview diff={iter.git_diff} filesChanged={iter.git_files_changed} />
            </div>
          </div>
        )
      })}
    </div>
  )
}
