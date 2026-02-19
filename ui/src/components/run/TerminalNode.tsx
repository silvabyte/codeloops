import { memo } from 'react'
import { Handle, Position, type NodeProps } from '@xyflow/react'
import { cn } from '@/lib/utils'
import type { TerminalNodeData } from '@/lib/flow-layout'

/**
 * Terminal/end node for the flow visualization.
 * Shows the session outcome: Completed, Error, Max Iterations, etc.
 */
export const TerminalNode = memo(function TerminalNode({
  data,
}: NodeProps & { data: TerminalNodeData }) {
  const { outcome, label } = data

  const colorClasses: Record<string, string> = {
    success: 'border-success/50 bg-success/10 text-success',
    failed: 'border-destructive/50 bg-destructive/10 text-destructive',
    max_iterations_reached: 'border-amber/50 bg-amber-glow text-amber',
    user_interrupted: 'border-amber/50 bg-amber-glow text-amber',
    starting: 'border-border bg-elevated/50 text-dim',
  }

  return (
    <div
      className={cn(
        'rounded-lg border px-4 py-3 flex items-center justify-center min-w-[140px]',
        colorClasses[outcome] ?? 'border-border bg-surface text-muted-foreground',
      )}
    >
      <Handle type="target" position={Position.Left} className="!bg-border !w-2 !h-2" />
      <span className="text-sm font-semibold">{label}</span>
    </div>
  )
})
