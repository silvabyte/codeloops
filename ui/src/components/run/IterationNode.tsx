import { memo } from 'react'
import { Handle, Position, type NodeProps } from '@xyflow/react'
import { cn } from '@/lib/utils'
import type { IterationNodeData } from '@/lib/flow-layout'
import type { IterationPhase } from '@/api/types'

/** Sub-node phases in display order */
const SUB_NODES = ['actor', 'diff', 'critic'] as const
type SubNode = (typeof SUB_NODES)[number]

/** Map phase to sub-node completion state */
function subNodeState(
  phase: string,
  subNode: SubNode,
): 'inactive' | 'active' | 'completed' | 'error' {
  const phaseMap: Record<SubNode, { activeAt: IterationPhase[]; completeAt: IterationPhase[] }> = {
    actor: {
      activeAt: ['actor_started'],
      completeAt: ['actor_completed', 'diff_captured', 'critic_started', 'critic_completed'],
    },
    diff: {
      activeAt: ['actor_completed'],
      completeAt: ['diff_captured', 'critic_started', 'critic_completed'],
    },
    critic: {
      activeAt: ['critic_started'],
      completeAt: ['critic_completed'],
    },
  }

  const config = phaseMap[subNode]
  if (config.completeAt.includes(phase as IterationPhase)) return 'completed'
  if (config.activeAt.includes(phase as IterationPhase)) return 'active'
  return 'inactive'
}

function subNodeLabel(subNode: SubNode): string {
  return subNode.charAt(0).toUpperCase() + subNode.slice(1)
}

interface SubNodeChipProps {
  subNode: SubNode
  state: 'inactive' | 'active' | 'completed' | 'error'
  onClick: () => void
}

function SubNodeChip({ subNode, state, onClick }: SubNodeChipProps) {
  return (
    <button
      onClick={(e) => {
        e.stopPropagation()
        onClick()
      }}
      className={cn(
        'px-2.5 py-1 rounded text-xs font-medium transition-all cursor-pointer',
        'border border-transparent',
        // Completed
        state === 'completed' && 'bg-success/15 text-success border-success/30',
        // Active (pulsing)
        state === 'active' && 'bg-amber-glow text-amber border-amber/40 node-pulse',
        // Inactive
        state === 'inactive' && 'bg-elevated/50 text-dim border-border-subtle',
        // Error
        state === 'error' && 'bg-destructive/15 text-destructive border-destructive/30',
      )}
    >
      {state === 'completed' && <span className="mr-1">&#10003;</span>}
      {state === 'active' && <span className="mr-1">&#9654;</span>}
      {subNodeLabel(subNode)}
    </button>
  )
}

/**
 * Custom react-flow node for an iteration gate.
 * Contains 3 clickable sub-nodes: Actor -> Diff -> Critic
 */
export const IterationNode = memo(function IterationNode({
  data,
}: NodeProps & { data: IterationNodeData }) {
  const { iteration, iterationNumber, isCurrent } = data
  const phase = iteration.phase

  // Metadata
  const duration = iteration.actorDurationSecs
  const filesChanged = iteration.gitFilesChanged
  const decision = iteration.criticDecision

  return (
    <div
      className={cn(
        'rounded-lg border bg-surface px-3 py-2.5 min-w-[260px]',
        isCurrent ? 'border-amber/50 shadow-[0_0_12px_var(--color-amber-glow)]' : 'border-border',
      )}
    >
      <Handle type="target" position={Position.Left} className="!bg-border !w-2 !h-2" />

      {/* Header */}
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs font-semibold text-foreground">
          Iteration {iterationNumber}
        </span>
        <span className="text-[10px] text-dim">
          {duration != null ? `${duration.toFixed(1)}s` : ''}
          {filesChanged != null ? ` / ${filesChanged} files` : ''}
        </span>
      </div>

      {/* Sub-nodes row */}
      <div className="flex items-center gap-1.5">
        {SUB_NODES.map((subNode, i) => (
          <div key={subNode} className="flex items-center">
            <SubNodeChip
              subNode={subNode}
              state={subNodeState(phase, subNode)}
              onClick={() => {
                // Dispatch a custom event for the parent to handle
                window.dispatchEvent(
                  new CustomEvent('run-node-click', {
                    detail: { iterationNumber, subNode, phase },
                  }),
                )
              }}
            />
            {i < SUB_NODES.length - 1 && (
              <span className="text-dim mx-0.5 text-[10px]">&#8594;</span>
            )}
          </div>
        ))}
      </div>

      {/* Decision summary (only if critic completed) */}
      {decision && (
        <div className="mt-1.5 text-[10px] text-muted-foreground truncate">
          {decision.toLowerCase() === 'done'
            ? 'accept'
            : decision.toLowerCase() === 'continue'
              ? 'revise'
              : decision.toLowerCase()}
        </div>
      )}

      <Handle type="source" position={Position.Right} className="!bg-border !w-2 !h-2" />
    </div>
  )
})
