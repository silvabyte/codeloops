import type { Node, Edge } from '@xyflow/react'
import type { Iteration } from '@/api/types'

/** Layout constants */
const GATE_WIDTH = 280
const GATE_HEIGHT = 120
const GATE_GAP = 60
const TERMINAL_WIDTH = 160
const TERMINAL_HEIGHT = 80
const START_X = 40
const START_Y = 40

export interface IterationNodeData {
  iteration: Iteration
  iterationNumber: number
  isActive: boolean
  isCurrent: boolean
}

export interface TerminalNodeData {
  outcome: string
  label: string
}

/**
 * Compute react-flow nodes and edges from iteration data.
 * Lays out iteration gates horizontally with a terminal node at the end.
 */
export function computeFlowLayout(
  iterations: Iteration[],
  sessionOutcome: string | null,
): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = []
  const edges: Edge[] = []

  iterations.forEach((iter, index) => {
    const x = START_X + index * (GATE_WIDTH + GATE_GAP)
    const y = START_Y
    const isLastIteration = index === iterations.length - 1
    const isActive = iter.phase !== 'critic_completed'

    const nodeId = `iteration-${iter.iterationNumber}`

    nodes.push({
      id: nodeId,
      type: 'iterationNode',
      position: { x, y },
      data: {
        iteration: iter,
        iterationNumber: iter.iterationNumber,
        isActive,
        isCurrent: isLastIteration && isActive,
      } satisfies IterationNodeData,
      width: GATE_WIDTH,
      height: GATE_HEIGHT,
    })

    // Edge from previous iteration to this one
    if (index > 0) {
      const prevId = `iteration-${iterations[index - 1].iterationNumber}`
      edges.push({
        id: `edge-${prevId}-${nodeId}`,
        source: prevId,
        target: nodeId,
        type: 'smoothstep',
        animated: false,
        style: { stroke: 'var(--color-border)', strokeWidth: 2 },
      })
    }

    // Terminal node after last completed iteration if session is done
    if (isLastIteration && sessionOutcome) {
      const terminalId = 'terminal'
      const terminalX = x + GATE_WIDTH + GATE_GAP
      const terminalY = START_Y + (GATE_HEIGHT - TERMINAL_HEIGHT) / 2

      nodes.push({
        id: terminalId,
        type: 'terminalNode',
        position: { x: terminalX, y: terminalY },
        data: {
          outcome: sessionOutcome,
          label: terminalLabel(sessionOutcome),
        } satisfies TerminalNodeData,
        width: TERMINAL_WIDTH,
        height: TERMINAL_HEIGHT,
      })

      edges.push({
        id: `edge-${nodeId}-terminal`,
        source: nodeId,
        target: terminalId,
        type: 'smoothstep',
        animated: false,
        style: { stroke: 'var(--color-border)', strokeWidth: 2 },
      })
    }
  })

  // Empty state: show a placeholder start node
  if (iterations.length === 0) {
    nodes.push({
      id: 'empty-start',
      type: 'terminalNode',
      position: { x: START_X, y: START_Y },
      data: {
        outcome: 'starting',
        label: 'Starting...',
      } satisfies TerminalNodeData,
      width: TERMINAL_WIDTH,
      height: TERMINAL_HEIGHT,
    })
  }

  return { nodes, edges }
}

function terminalLabel(outcome: string): string {
  switch (outcome) {
    case 'success':
      return 'Completed'
    case 'failed':
      return 'Error'
    case 'max_iterations_reached':
      return 'Max Iterations'
    case 'user_interrupted':
      return 'Interrupted'
    default:
      return outcome
  }
}

/**
 * Get the viewport bounds that fit all nodes, with padding.
 */
export function getViewportBounds(nodes: Node[]): {
  x: number
  y: number
  width: number
  height: number
} {
  if (nodes.length === 0) {
    return { x: 0, y: 0, width: 800, height: 300 }
  }

  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity

  for (const node of nodes) {
    const w = (node.width as number) || GATE_WIDTH
    const h = (node.height as number) || GATE_HEIGHT
    minX = Math.min(minX, node.position.x)
    minY = Math.min(minY, node.position.y)
    maxX = Math.max(maxX, node.position.x + w)
    maxY = Math.max(maxY, node.position.y + h)
  }

  const padding = 60
  return {
    x: minX - padding,
    y: minY - padding,
    width: maxX - minX + padding * 2,
    height: maxY - minY + padding * 2,
  }
}
