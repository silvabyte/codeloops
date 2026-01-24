import { cn } from '@/lib/utils'
import type { Iteration } from '@/api/types'

interface CriticTrailProps {
  iterations: Iteration[]
}

function decisionColor(decision: string): string {
  switch (decision.toUpperCase()) {
    case 'DONE': return 'border-success/50'
    case 'CONTINUE': return 'border-warning/50'
    default: return 'border-destructive/50'
  }
}

export function CriticTrail({ iterations }: CriticTrailProps) {
  if (iterations.length === 0) {
    return <div className="text-muted-foreground text-sm">No iterations yet.</div>
  }

  return (
    <div className="space-y-4">
      <div className="text-xs text-muted-foreground uppercase tracking-wider">
        Critic Feedback Trail
      </div>

      <div className="space-y-3">
        {iterations.map((iter) => (
          <div
            key={iter.iteration_number}
            className={cn('border-l-2 pl-4 py-2', decisionColor(iter.critic_decision))}
          >
            <div className="flex items-center gap-2 mb-1">
              <span className="text-xs font-medium text-muted-foreground">
                Iteration {iter.iteration_number}
              </span>
              <span className={cn(
                'text-xs px-1.5 py-0.5 rounded',
                iter.critic_decision === 'DONE' && 'bg-success/20 text-success',
                iter.critic_decision === 'CONTINUE' && 'bg-warning/20 text-warning',
                !['DONE', 'CONTINUE'].includes(iter.critic_decision) && 'bg-destructive/20 text-destructive',
              )}>
                {iter.critic_decision}
              </span>
            </div>
            {iter.feedback ? (
              <div className="text-sm whitespace-pre-wrap text-foreground/90">
                {iter.feedback.length > 500
                  ? <details>
                      <summary className="cursor-pointer">{iter.feedback.slice(0, 300)}...</summary>
                      <div className="mt-2">{iter.feedback}</div>
                    </details>
                  : iter.feedback
                }
              </div>
            ) : (
              <div className="text-sm text-muted-foreground italic">No feedback provided</div>
            )}
          </div>
        ))}
      </div>
    </div>
  )
}
