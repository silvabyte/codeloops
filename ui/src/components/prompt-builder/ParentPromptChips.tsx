import { useState, useEffect } from 'react'
import { cn } from '@/lib/utils'
import type { PromptSummary } from '@/lib/prompt-session'
import { getPromptById } from '@/lib/prompt-session'
import { X, Plus } from 'lucide-react'

interface ParentPromptChipsProps {
  parentIds: string[]
  onRemove: (id: string) => void
  onAdd: () => void
}

interface ParentInfo {
  id: string
  title: string
  loading: boolean
  error: boolean
}

export function ParentPromptChips({ parentIds, onRemove, onAdd }: ParentPromptChipsProps) {
  const [parentInfo, setParentInfo] = useState<ParentInfo[]>([])

  useEffect(() => {
    // Initialize with loading state for each parent
    setParentInfo(
      parentIds.map((id) => ({
        id,
        title: 'Loading...',
        loading: true,
        error: false,
      }))
    )

    // Fetch info for each parent
    parentIds.forEach(async (id) => {
      try {
        const prompt = await getPromptById(id)
        setParentInfo((prev) =>
          prev.map((p) =>
            p.id === id ? { ...p, title: prompt.title || 'Untitled', loading: false } : p
          )
        )
      } catch {
        setParentInfo((prev) =>
          prev.map((p) => (p.id === id ? { ...p, title: 'Not found', loading: false, error: true } : p))
        )
      }
    })
  }, [parentIds])

  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="text-xs text-muted-foreground">Extends:</span>
      {parentInfo.map((parent) => (
        <div
          key={parent.id}
          className={cn(
            'inline-flex items-center gap-1.5 px-2 py-1 rounded-md text-xs',
            'bg-amber-500/10 text-amber-500 border border-amber-500/20',
            parent.error && 'bg-destructive/10 text-destructive border-destructive/20'
          )}
        >
          <span className={cn('truncate max-w-[150px]', parent.loading && 'animate-pulse')}>
            {parent.title}
          </span>
          <button
            onClick={() => onRemove(parent.id)}
            className="p-0.5 hover:bg-amber-500/20 rounded transition-colors"
            aria-label={`Remove ${parent.title}`}
          >
            <X className="w-3 h-3" />
          </button>
        </div>
      ))}
      <button
        onClick={onAdd}
        className={cn(
          'inline-flex items-center gap-1 px-2 py-1 rounded-md text-xs',
          'text-muted-foreground hover:text-foreground',
          'border border-dashed border-border hover:border-foreground/50',
          'transition-colors'
        )}
      >
        <Plus className="w-3 h-3" />
        <span>Add Parent</span>
      </button>
    </div>
  )
}
