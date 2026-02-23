import { useEffect, useRef } from 'react'
import { cn } from '@/lib/utils'
import { useNodeOutput } from '@/hooks/useNodeOutput'
import { DiffViewer } from '@/components/DiffViewer'
import type { Iteration } from '@/api/types'

export type SelectedNode = {
  iterationNumber: number
  subNode: 'actor' | 'diff' | 'critic'
  phase: string
}

interface OutputPanelProps {
  selectedNode: SelectedNode
  iteration: Iteration | undefined
  sessionId: string
  onClose: () => void
}

/** Phase ordering for checking if a sub-node is complete */
const PHASE_INDEX: Record<string, number> = {
  actor_started: 0,
  actor_completed: 1,
  diff_captured: 2,
  critic_started: 3,
  critic_completed: 4,
}

function isSubNodeComplete(iterPhase: string, subNode: 'actor' | 'diff' | 'critic'): boolean {
  const phaseIdx = PHASE_INDEX[iterPhase] ?? 0
  switch (subNode) {
    case 'actor':
      return phaseIdx >= 1 // actor_completed or later
    case 'diff':
      return phaseIdx >= 2 // diff_captured or later
    case 'critic':
      return phaseIdx >= 4 // critic_completed
    default:
      return false
  }
}

function isSubNodeActive(iterPhase: string, subNode: 'actor' | 'diff' | 'critic'): boolean {
  switch (subNode) {
    case 'actor':
      return iterPhase === 'actor_started'
    case 'diff':
      return iterPhase === 'actor_completed'
    case 'critic':
      return iterPhase === 'critic_started'
    default:
      return false
  }
}

/**
 * Side panel that shows output for a clicked node.
 * - Completed actor/critic: full output from DB
 * - Completed diff: renders with DiffViewer
 * - In-progress actor/critic: live SSE stdout stream
 * - Inactive: "Waiting..." placeholder
 */
export function OutputPanel({ selectedNode, iteration, sessionId, onClose }: OutputPanelProps) {
  const { iterationNumber, subNode } = selectedNode
  const iterPhase = iteration?.phase ?? 'actor_started'

  const isComplete = isSubNodeComplete(iterPhase, subNode)
  const isActive = isSubNodeActive(iterPhase, subNode)
  const isWaiting = !isComplete && !isActive

  // Live streaming for in-progress nodes
  const shouldStream = isActive && (subNode === 'actor' || subNode === 'critic')
  const { lines, isStreaming } = useNodeOutput(
    sessionId,
    iterationNumber,
    subNode as 'actor' | 'critic',
    shouldStream,
  )

  const label = `Iteration ${iterationNumber} — ${subNode.charAt(0).toUpperCase() + subNode.slice(1)}`

  return (
    <div className="h-full flex flex-col border-l border-border bg-surface">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border bg-elevated/30">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-foreground">{label}</span>
          {isStreaming && (
            <span className="text-[10px] px-1.5 py-0.5 rounded bg-amber-glow text-amber">
              LIVE
            </span>
          )}
        </div>
        <button
          onClick={onClose}
          className="text-muted-foreground hover:text-foreground transition-colors text-lg leading-none"
          aria-label="Close panel"
        >
          &times;
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {isWaiting && <WaitingState />}
        {isComplete && subNode === 'actor' && <CompletedActorOutput iteration={iteration} />}
        {isComplete && subNode === 'critic' && <CompletedCriticOutput iteration={iteration} />}
        {isComplete && subNode === 'diff' && <CompletedDiffOutput iteration={iteration} />}
        {isActive && shouldStream && <LiveOutput lines={lines} isStreaming={isStreaming} />}
        {isActive && subNode === 'diff' && <WaitingState message="Capturing diff..." />}
      </div>
    </div>
  )
}

function WaitingState({ message = 'Waiting...' }: { message?: string }) {
  return (
    <div className="flex items-center justify-center h-full text-dim text-sm">
      {message}
    </div>
  )
}

function CompletedActorOutput({ iteration }: { iteration: Iteration | undefined }) {
  if (!iteration) return <WaitingState />

  return (
    <div className="p-4 space-y-3">
      {iteration.actorOutput && (
        <TerminalBlock label="stdout" content={iteration.actorOutput} />
      )}
      {iteration.actorStderr && (
        <TerminalBlock label="stderr" content={iteration.actorStderr} variant="error" />
      )}
      {!iteration.actorOutput && !iteration.actorStderr && (
        <div className="text-dim text-sm">No output captured.</div>
      )}
    </div>
  )
}

function CompletedCriticOutput({ iteration }: { iteration: Iteration | undefined }) {
  if (!iteration) return <WaitingState />

  return (
    <div className="p-4 space-y-3">
      {iteration.criticDecision && (
        <div className="text-xs text-muted-foreground">
          Decision:{' '}
          <span className={cn(
            'font-medium',
            iteration.criticDecision === 'DONE' && 'text-success',
            iteration.criticDecision === 'CONTINUE' && 'text-amber',
          )}>
            {iteration.criticDecision}
          </span>
        </div>
      )}
      {iteration.feedback && (
        <TerminalBlock label="feedback" content={iteration.feedback} />
      )}
      {!iteration.feedback && (
        <div className="text-dim text-sm">No feedback provided.</div>
      )}
    </div>
  )
}

function CompletedDiffOutput({ iteration }: { iteration: Iteration | undefined }) {
  if (!iteration?.gitDiff) {
    return (
      <div className="p-4 text-dim text-sm">No diff available.</div>
    )
  }

  return (
    <div className="p-2">
      <DiffViewer diff={iteration.gitDiff} />
    </div>
  )
}

function LiveOutput({
  lines,
  isStreaming,
}: {
  lines: { line: string; stream: string }[]
  isStreaming: boolean
}) {
  const scrollRef = useRef<HTMLPreElement>(null)

  // Auto-scroll to bottom as new lines arrive
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [lines.length])

  return (
    <div className="h-full flex flex-col">
      <pre
        ref={scrollRef}
        className="flex-1 p-4 text-xs overflow-y-auto overflow-x-auto font-mono bg-background"
      >
        {lines.map((l, i) => (
          <div
            key={i}
            className={cn(l.stream === 'stderr' ? 'text-destructive/80' : 'text-foreground/90')}
          >
            {l.line}
          </div>
        ))}
        {isStreaming && (
          <span className="text-amber animate-pulse">&#9608;</span>
        )}
      </pre>
    </div>
  )
}

function TerminalBlock({
  label,
  content,
  variant = 'default',
}: {
  label: string
  content: string
  variant?: 'default' | 'error'
}) {
  return (
    <div>
      <div className="text-[10px] text-dim uppercase tracking-wider mb-1">{label}</div>
      <pre
        className={cn(
          'p-3 rounded-lg text-xs overflow-x-auto max-h-[500px] overflow-y-auto font-mono',
          variant === 'error' ? 'bg-destructive/5 text-destructive/80' : 'bg-background text-foreground/90',
        )}
      >
        {content}
      </pre>
    </div>
  )
}
