import { useState, useCallback } from 'react'
import ReactMarkdown from 'react-markdown'
import { cn } from '@/lib/utils'
import { markdownPreview } from '@/lib/markdown-styles'
import { FileText, Save, Copy, Download } from 'lucide-react'
import { ParentPromptChips } from './ParentPromptChips'
import { ParentPromptSelector } from './ParentPromptSelector'
import { InheritedContentPreview } from './InheritedContentPreview'

interface PreviewPanelProps {
  content: string
  onContentChange: (content: string) => void
  onSave: () => void
  onCopy: () => void
  onDownload: () => void
  isSaving?: boolean
  isStreaming?: boolean
  promptId?: string
  parentIds?: string[]
  onParentIdsChange?: (parentIds: string[]) => void
}

export function PreviewPanel({
  content,
  onContentChange,
  onSave,
  onCopy,
  onDownload,
  isSaving,
  isStreaming,
  promptId,
  parentIds = [],
  onParentIdsChange,
}: PreviewPanelProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [showParentSelector, setShowParentSelector] = useState(false)

  const handleAddParent = useCallback(
    (id: string) => {
      if (onParentIdsChange && !parentIds.includes(id)) {
        onParentIdsChange([...parentIds, id])
      }
    },
    [parentIds, onParentIdsChange]
  )

  const handleRemoveParent = useCallback(
    (id: string) => {
      if (onParentIdsChange) {
        onParentIdsChange(parentIds.filter((pid) => pid !== id))
      }
    },
    [parentIds, onParentIdsChange]
  )

  const handleCopy = useCallback(async () => {
    await navigator.clipboard.writeText(content)
    onCopy()
  }, [content, onCopy])

  const handleDownload = useCallback(() => {
    const blob = new Blob([content], { type: 'text/markdown' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'prompt.md'
    a.click()
    URL.revokeObjectURL(url)
    onDownload()
  }, [content, onDownload])

  // Calculate exclude IDs for parent selector (current prompt + already selected parents)
  const excludeIds = promptId ? [promptId, ...parentIds] : parentIds

  return (
    <div className="h-full flex flex-col border-l border-border bg-card">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        {/* Left: file icon + name */}
        <div className="flex items-center gap-2">
          <FileText className="w-4 h-4 text-dim" />
          <span className="text-sm font-mono font-medium text-muted-foreground">prompt.md</span>
        </div>

        {/* Center: Segmented toggle */}
        <div className="flex items-center rounded-lg border border-border bg-background p-0.5">
          <button
            onClick={() => setIsEditing(false)}
            className={cn(
              'text-xs px-3 py-1 rounded-md transition-all',
              !isEditing
                ? 'bg-elevated shadow-sm text-foreground'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >
            Preview
          </button>
          <button
            onClick={() => setIsEditing(true)}
            className={cn(
              'text-xs px-3 py-1 rounded-md transition-all',
              isEditing
                ? 'bg-elevated shadow-sm text-foreground'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >
            Edit
          </button>
        </div>

        {/* Right: spacer for balance */}
        <div className="w-20" />
      </div>

      {/* Referenced Prompts Section */}
      {onParentIdsChange && (
        <div className="px-4 py-3 border-b border-border">
          <ParentPromptChips
            parentIds={parentIds}
            onRemove={handleRemoveParent}
            onAdd={() => setShowParentSelector(true)}
          />
        </div>
      )}

      {/* Inherited Content Preview */}
      {promptId && parentIds.length > 0 && (
        <InheritedContentPreview promptId={promptId} parentIds={parentIds} />
      )}

      {/* Streaming indicator */}
      {isStreaming && (
        <div className="flex items-center gap-2 px-4 py-2 border-b border-border bg-amber/5">
          <div className="flex gap-1">
            <div className="w-1.5 h-1.5 rounded-full bg-amber animate-pulse" />
            <div className="w-1.5 h-1.5 rounded-full bg-amber animate-pulse" style={{ animationDelay: '0.2s' }} />
            <div className="w-1.5 h-1.5 rounded-full bg-amber animate-pulse" style={{ animationDelay: '0.4s' }} />
          </div>
          <span className="text-xs text-amber">Generating prompt...</span>
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4">
        {isEditing ? (
          <textarea
            value={content}
            onChange={(e) => onContentChange(e.target.value)}
            className={cn(
              'w-full h-full bg-transparent text-foreground text-sm',
              'resize-none outline-none leading-relaxed',
              'font-mono'
            )}
          />
        ) : content ? (
          <div
            className={cn(
              'text-sm leading-relaxed max-w-none text-foreground/90',
              markdownPreview
            )}
          >
            <ReactMarkdown>{content}</ReactMarkdown>
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center h-full gap-3 text-center">
            <div className="w-12 h-12 rounded-lg border border-border flex items-center justify-center">
              <FileText className="w-6 h-6 text-dim" />
            </div>
            <p className="text-sm text-muted-foreground/50">
              Prompt will appear here as you chat...
            </p>
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-2 px-4 py-3 border-t border-border">
        <button
          onClick={onSave}
          disabled={isSaving || !content}
          className={cn(
            'flex-1 inline-flex items-center justify-center gap-2 text-sm py-2 rounded-lg transition-colors',
            'bg-amber text-background font-medium',
            'hover:bg-amber-bright',
            'disabled:opacity-50 disabled:cursor-not-allowed'
          )}
        >
          <Save className="w-4 h-4" />
          {isSaving ? 'Saving...' : 'Save'}
        </button>
        <button
          onClick={handleCopy}
          disabled={!content}
          className={cn(
            'p-2 rounded-lg transition-colors',
            'text-muted-foreground hover:text-foreground hover:bg-elevated',
            'disabled:opacity-50 disabled:cursor-not-allowed'
          )}
          aria-label="Copy to clipboard"
          title="Copy to clipboard"
        >
          <Copy className="w-4 h-4" />
        </button>
        <button
          onClick={handleDownload}
          disabled={!content}
          className={cn(
            'p-2 rounded-lg transition-colors',
            'text-muted-foreground hover:text-foreground hover:bg-elevated',
            'disabled:opacity-50 disabled:cursor-not-allowed'
          )}
          aria-label="Download as file"
          title="Download as file"
        >
          <Download className="w-4 h-4" />
        </button>
      </div>

      {/* Reference Selector Modal */}
      <ParentPromptSelector
        isOpen={showParentSelector}
        onClose={() => setShowParentSelector(false)}
        onSelect={handleAddParent}
        excludeIds={excludeIds}
      />
    </div>
  )
}
