import { useState, useCallback } from 'react'
import { cn } from '@/lib/utils'

interface PreviewPanelProps {
  content: string
  onContentChange: (content: string) => void
  onSave: () => void
  onCopy: () => void
  onDownload: () => void
  isSaving?: boolean
}

export function PreviewPanel({
  content,
  onContentChange,
  onSave,
  onCopy,
  onDownload,
  isSaving,
}: PreviewPanelProps) {
  const [isEditing, setIsEditing] = useState(false)

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

  return (
    <div className="h-full flex flex-col border-l border-border bg-card">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <span className="text-sm text-muted-foreground">prompt.md</span>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsEditing(!isEditing)}
            className={cn(
              'text-xs px-2 py-1 rounded transition-colors',
              isEditing
                ? 'text-foreground bg-secondary'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >
            {isEditing ? 'Preview' : 'Edit'}
          </button>
        </div>
      </div>

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
        ) : (
          <div className="text-sm text-foreground/90 whitespace-pre-wrap leading-relaxed font-mono">
            {content || (
              <span className="text-muted-foreground/50 italic">
                Prompt will appear here as you chat...
              </span>
            )}
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-2 px-4 py-3 border-t border-border">
        <button
          onClick={onSave}
          disabled={isSaving || !content}
          className={cn(
            'flex-1 text-sm py-2 rounded transition-colors',
            'bg-primary text-primary-foreground',
            'hover:bg-primary/90',
            'disabled:opacity-50 disabled:cursor-not-allowed'
          )}
        >
          {isSaving ? 'Saving...' : 'Save'}
        </button>
        <button
          onClick={handleCopy}
          disabled={!content}
          className={cn(
            'text-sm px-3 py-2 rounded transition-colors',
            'text-muted-foreground hover:text-foreground',
            'hover:bg-secondary',
            'disabled:opacity-50 disabled:cursor-not-allowed'
          )}
        >
          Copy
        </button>
        <button
          onClick={handleDownload}
          disabled={!content}
          className={cn(
            'text-sm px-3 py-2 rounded transition-colors',
            'text-muted-foreground hover:text-foreground',
            'hover:bg-secondary',
            'disabled:opacity-50 disabled:cursor-not-allowed'
          )}
        >
          Download
        </button>
      </div>
    </div>
  )
}
