import { cn } from '@/lib/utils'
import type { SessionStats } from '@/api/types'

interface HealthPulseProps {
  stats: SessionStats | null
  loading: boolean
}

export function HealthPulse({ stats, loading }: HealthPulseProps) {
  if (loading || !stats) {
    return (
      <div className="flex items-center gap-3 h-8">
        <div className="w-3 h-3 rounded-full bg-muted animate-pulse" />
        <div className="h-5 w-48 bg-muted rounded animate-pulse" />
      </div>
    )
  }

  const rate = stats.success_rate
  const percentage = Math.round(rate * 100)

  // Determine health status
  const isHealthy = rate >= 0.7
  const isWarning = rate >= 0.4 && rate < 0.7
  const isCritical = rate < 0.4

  const dotClasses = cn(
    'w-3 h-3 rounded-full',
    isHealthy && 'bg-green-500 shadow-[0_0_12px_rgba(34,197,94,0.4)]',
    isWarning && 'bg-amber-500 shadow-[0_0_12px_rgba(245,158,11,0.4)]',
    isCritical && 'bg-red-500 shadow-[0_0_12px_rgba(239,68,68,0.4)]'
  )

  const textClasses = cn(
    'font-mono text-xl tabular-nums',
    isHealthy && 'text-green-500',
    isWarning && 'text-amber-500',
    isCritical && 'text-red-500'
  )

  return (
    <div className="flex items-center gap-3">
      <div className={dotClasses} />
      <span className={textClasses}>{percentage}%</span>
      <span className="text-muted-foreground text-sm">
        success rate across {stats.total_sessions} session{stats.total_sessions !== 1 ? 's' : ''}
      </span>
    </div>
  )
}
