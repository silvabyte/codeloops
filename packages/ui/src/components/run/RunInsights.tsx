import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  ReactFlow,
  Background,
  Controls,
  useReactFlow,
  ReactFlowProvider,
  type NodeTypes,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'

import { useRunState } from '@/hooks/useRunState'
import { computeFlowLayout } from '@/lib/flow-layout'
import { IterationNode } from './IterationNode'
import { TerminalNode } from './TerminalNode'
import { OutputPanel, type SelectedNode } from './OutputPanel'

const nodeTypes: NodeTypes = {
  iterationNode: IterationNode,
  terminalNode: TerminalNode,
}

interface RunInsightsProps {
  sessionId: string
  pollInterval?: number
}

/**
 * Full-screen react-flow canvas showing the actor-critic iteration pipeline.
 * Each iteration is a gate with clickable Actor -> Diff -> Critic sub-nodes.
 */
function RunInsightsInner({ sessionId, pollInterval = 3000 }: RunInsightsProps) {
  const { iterations, isLive, isComplete, outcome, session, loading, error } =
    useRunState(sessionId, pollInterval)

  const [selectedNode, setSelectedNode] = useState<SelectedNode | null>(null)
  const { fitView } = useReactFlow()

  // Compute flow layout from iterations
  const { nodes, edges } = useMemo(
    () => computeFlowLayout(iterations, outcome),
    [iterations, outcome],
  )

  // Auto-fit when nodes change
  useEffect(() => {
    if (nodes.length > 0) {
      // Small delay to let react-flow render the nodes first
      const timer = setTimeout(() => fitView({ padding: 0.2, duration: 300 }), 100)
      return () => clearTimeout(timer)
    }
  }, [nodes.length, fitView])

  // Listen for sub-node click events from IterationNode
  useEffect(() => {
    function handleNodeClick(e: Event) {
      const detail = (e as CustomEvent).detail as {
        iterationNumber: number
        subNode: 'actor' | 'diff' | 'critic'
        phase: string
      }
      setSelectedNode(detail)
    }
    window.addEventListener('run-node-click', handleNodeClick)
    return () => window.removeEventListener('run-node-click', handleNodeClick)
  }, [])

  const handleClosePanel = useCallback(() => setSelectedNode(null), [])

  // Find the iteration for the selected node
  const selectedIteration = selectedNode
    ? iterations.find((i) => i.iterationNumber === selectedNode.iterationNumber)
    : undefined

  if (loading && !session) {
    return (
      <div className="flex items-center justify-center h-full text-dim">
        Loading run data...
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full text-destructive text-sm">
        {error}
      </div>
    )
  }

  return (
    <div className="flex h-full">
      {/* Canvas */}
      <div className="flex-1 relative">
        {/* Status badge */}
        <div className="absolute top-3 left-3 z-10 flex items-center gap-2">
          {isLive && (
            <span className="text-[10px] px-2 py-1 rounded bg-amber-glow text-amber border border-amber/30 font-medium">
              &#9679; LIVE
            </span>
          )}
          {isComplete && (
            <span className="text-[10px] px-2 py-1 rounded bg-elevated text-muted-foreground border border-border font-medium">
              {outcome === 'success' ? 'COMPLETED' : outcome?.toUpperCase()}
            </span>
          )}
          <span className="text-[10px] text-dim">
            {iterations.length} iteration{iterations.length !== 1 ? 's' : ''}
          </span>
        </div>

        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          fitView
          fitViewOptions={{ padding: 0.2 }}
          panOnDrag
          zoomOnScroll
          zoomOnPinch
          nodesDraggable={false}
          nodesConnectable={false}
          elementsSelectable={false}
          minZoom={0.3}
          maxZoom={2}
          proOptions={{ hideAttribution: true }}
          className="bg-background"
        >
          <Background
            color="var(--color-border-subtle)"
            gap={20}
            size={1}
          />
          <Controls
            className="!bg-surface !border-border !shadow-none [&_button]:!bg-elevated [&_button]:!border-border [&_button]:!text-muted-foreground [&_button:hover]:!bg-hover"
            showInteractive={false}
          />
        </ReactFlow>
      </div>

      {/* Side panel */}
      {selectedNode && (
        <div className="w-[40%] min-w-[320px] max-w-[600px]">
          <OutputPanel
            selectedNode={selectedNode}
            iteration={selectedIteration}
            sessionId={sessionId}
            onClose={handleClosePanel}
          />
        </div>
      )}
    </div>
  )
}

/**
 * RunInsights wrapped with ReactFlowProvider.
 * This is the top-level component to render in the Run tab.
 */
export function RunInsights(props: RunInsightsProps) {
  return (
    <ReactFlowProvider>
      <RunInsightsInner {...props} />
    </ReactFlowProvider>
  )
}
