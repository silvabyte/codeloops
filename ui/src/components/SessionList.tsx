import { useNavigate } from 'react-router-dom'
import { cn, formatDuration, formatDate, formatRelativeTime } from '@/lib/utils'
import type { SessionSummary } from '@/api/types'

interface SessionListProps {
  sessions: SessionSummary[]
  loading: boolean
}

function StatusDot({ outcome }: { outcome: string | null }) {
  const isActive = outcome === null
  const isSuccess = outcome === 'success'
  const isFailed = outcome === 'failed'
  const isWarning = outcome === 'interrupted' || outcome === 'max_iterations_reached'

  return (
    <div
      className={cn(
        'w-2.5 h-2.5 rounded-full shrink-0',
        isActive && 'bg-amber-500 animate-pulse shadow-[0_0_8px_rgba(245,158,11,0.5)]',
        isSuccess && 'bg-green-500',
        isFailed && 'bg-red-500',
        isWarning && 'bg-amber-500'
      )}
    />
  )
}

export function SessionList({ sessions, loading }: SessionListProps) {
  const navigate = useNavigate()

  if (loading) {
    return (
      <div className="space-y-3">
        {[...Array(5)].map((_, i) => (
          <div
            key={i}
            className="rounded-lg bg-surface/50 border border-transparent p-4 animate-pulse"
          >
            <div className="flex items-start gap-3">
              <div className="w-2.5 h-2.5 rounded-full bg-muted mt-1.5" />
              <div className="flex-1 space-y-2">
                <div className="h-5 bg-muted rounded w-3/4" />
                <div className="h-4 bg-muted rounded w-1/3" />
              </div>
            </div>
          </div>
        ))}
      </div>
    )
  }

  if (sessions.length === 0) {
    return (
      <div className="text-center py-16 text-muted-foreground">
        No sessions found.
      </div>
    )
  }

  return (
    <div className="space-y-2">
      {sessions.map((s) => (
        <div
          key={s.id}
          onClick={() => navigate(`/sessions/${encodeURIComponent(s.id)}`)}
          className={cn(
            'rounded-lg bg-surface/50 border border-transparent p-4 cursor-pointer transition-colors',
            'hover:bg-surface hover:border-border-subtle',
            'focus:bg-surface focus:border-amber-dim'
          )}
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === 'Enter') navigate(`/sessions/${encodeURIComponent(s.id)}`)
          }}
        >
          <div className="flex items-start gap-3">
            <StatusDot outcome={s.outcome} />
            <div className="flex-1 min-w-0">
              <div className="flex items-start justify-between gap-4">
                <p className="text-foreground leading-snug">{s.promptPreview}</p>
                <span
                  className="text-xs text-muted-foreground shrink-0"
                  title={formatDate(s.timestamp)}
                >
                  {formatRelativeTime(s.timestamp)}
                </span>
              </div>
              <div className="flex items-center gap-2 mt-1.5 text-xs text-muted-foreground">
                <span>{s.project}</span>
                {s.durationSecs && (
                  <>
                    <span className="opacity-50">/</span>
                    <span>{formatDuration(s.durationSecs)}</span>
                  </>
                )}
              </div>
            </div>
          </div>
        </div>
      ))}
    </div>
  )
}
