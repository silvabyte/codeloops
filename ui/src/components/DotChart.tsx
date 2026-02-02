import { cn } from '@/lib/utils'
import type { ProjectStats } from '@/api/types'

interface DotChartProps {
  projects: ProjectStats[]
  maxDots?: number
}

export function DotChart({ projects, maxDots = 16 }: DotChartProps) {
  if (projects.length === 0) {
    return (
      <div className="text-center py-8 text-muted-foreground text-sm">
        No project data available.
      </div>
    )
  }

  return (
    <div className="space-y-3">
      {projects.map((p) => {
        const successCount = Math.round(p.total * p.success_rate)
        const failCount = p.total - successCount

        // Scale dots if total exceeds maxDots
        const scale = p.total > maxDots ? maxDots / p.total : 1
        const displaySuccess = Math.round(successCount * scale)
        const displayFail = Math.round(failCount * scale)

        return (
          <div key={p.project} className="flex items-center gap-4">
            <span className="text-sm w-32 truncate" title={p.project}>
              {p.project}
            </span>
            <div className="flex gap-0.5 flex-1">
              {[...Array(displaySuccess)].map((_, i) => (
                <div
                  key={`s-${i}`}
                  className="w-2 h-2 rounded-full bg-green-500"
                />
              ))}
              {[...Array(displayFail)].map((_, i) => (
                <div
                  key={`f-${i}`}
                  className={cn(
                    'w-2 h-2 rounded-full',
                    'border border-amber-500/50 bg-transparent'
                  )}
                />
              ))}
            </div>
            <span className="text-xs text-muted-foreground w-28 text-right tabular-nums">
              {p.total} runs / {Math.round(p.success_rate * 100)}%
            </span>
          </div>
        )
      })}
    </div>
  )
}
