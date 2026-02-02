import { useState } from 'react'
import ReactMarkdown from 'react-markdown'
import { cn, formatDuration } from '@/lib/utils'
import type { Iteration } from '@/api/types'
import { ContentBlock } from './ContentBlock'
import { CopyButton } from './CopyButton'

const markdownStyles = cn(
  'text-sm leading-relaxed max-w-none',
  '[&_p]:mb-3 [&_p:last-child]:mb-0',
  '[&_pre]:bg-elevated [&_pre]:p-3 [&_pre]:rounded-lg [&_pre]:overflow-x-auto [&_pre]:my-3',
  '[&_code]:bg-elevated [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-amber [&_code]:text-xs',
  '[&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_pre_code]:text-foreground',
  '[&_a]:text-cyan [&_a]:no-underline hover:[&_a]:underline',
  '[&_ul]:list-disc [&_ul]:pl-5 [&_ul]:my-3',
  '[&_ol]:list-decimal [&_ol]:pl-5 [&_ol]:my-3',
  '[&_li]:mb-1',
  '[&_strong]:text-foreground [&_strong]:font-semibold',
  '[&_h1]:text-lg [&_h1]:font-semibold [&_h1]:text-foreground [&_h1]:mb-3 [&_h1]:mt-4 [&_h1:first-child]:mt-0',
  '[&_h2]:text-base [&_h2]:font-semibold [&_h2]:text-foreground [&_h2]:mb-2 [&_h2]:mt-4 [&_h2:first-child]:mt-0',
  '[&_h3]:text-sm [&_h3]:font-semibold [&_h3]:text-foreground [&_h3]:mb-2 [&_h3]:mt-3 [&_h3:first-child]:mt-0',
  '[&_blockquote]:border-l-2 [&_blockquote]:border-amber/50 [&_blockquote]:pl-3 [&_blockquote]:italic [&_blockquote]:my-3'
)

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
                markdown
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
                    <div className={markdownStyles}>
                      <ReactMarkdown>{iter.feedback}</ReactMarkdown>
                    </div>
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
