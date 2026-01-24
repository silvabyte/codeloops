import { useNavigate } from 'react-router-dom'
import { cn, formatDate, formatDuration } from '@/lib/utils'
import type { SessionSummary } from '@/api/types'

interface SessionTableProps {
  sessions: SessionSummary[]
  loading: boolean
}

function OutcomeBadge({ outcome }: { outcome: string | null }) {
  const label = outcome || 'active'
  const colors: Record<string, string> = {
    success: 'bg-success/20 text-success',
    failed: 'bg-destructive/20 text-destructive',
    active: 'bg-primary/20 text-primary',
    interrupted: 'bg-warning/20 text-warning',
    max_iterations_reached: 'bg-warning/20 text-warning',
  }

  return (
    <span className={cn('px-2 py-0.5 rounded-full text-xs font-medium', colors[label] || 'bg-muted text-muted-foreground')}>
      {label}
    </span>
  )
}

export function SessionTable({ sessions, loading }: SessionTableProps) {
  const navigate = useNavigate()

  if (loading) {
    return (
      <div className="space-y-2">
        {[...Array(5)].map((_, i) => (
          <div key={i} className="h-12 bg-card rounded-lg border border-border animate-pulse" />
        ))}
      </div>
    )
  }

  if (sessions.length === 0) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        No sessions found.
      </div>
    )
  }

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border bg-secondary/50">
            <th className="text-left px-4 py-2 text-muted-foreground font-medium">Time</th>
            <th className="text-left px-4 py-2 text-muted-foreground font-medium">Outcome</th>
            <th className="text-left px-4 py-2 text-muted-foreground font-medium">Iters</th>
            <th className="text-left px-4 py-2 text-muted-foreground font-medium">Duration</th>
            <th className="text-left px-4 py-2 text-muted-foreground font-medium">Project</th>
            <th className="text-left px-4 py-2 text-muted-foreground font-medium">Prompt</th>
          </tr>
        </thead>
        <tbody>
          {sessions.map((s) => (
            <tr
              key={s.id}
              onClick={() => navigate(`/sessions/${encodeURIComponent(s.id)}`)}
              className="border-b border-border hover:bg-secondary/30 cursor-pointer transition-colors"
            >
              <td className="px-4 py-2.5 whitespace-nowrap">{formatDate(s.timestamp)}</td>
              <td className="px-4 py-2.5"><OutcomeBadge outcome={s.outcome} /></td>
              <td className="px-4 py-2.5">{s.iterations}</td>
              <td className="px-4 py-2.5 whitespace-nowrap">
                {s.duration_secs ? formatDuration(s.duration_secs) : '...'}
              </td>
              <td className="px-4 py-2.5 text-muted-foreground">{s.project}</td>
              <td className="px-4 py-2.5 text-muted-foreground truncate max-w-xs">
                {s.prompt_preview}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
