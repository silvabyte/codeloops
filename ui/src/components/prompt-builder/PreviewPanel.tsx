import { useState, useCallback } from 'react'
import ReactMarkdown from 'react-markdown'
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
        ) : content ? (
          <div
            className={cn(
              'text-sm leading-relaxed max-w-none text-foreground/90',
              // Markdown element styling
              '[&_p]:mb-3 [&_p:last-child]:mb-0',
              '[&_pre]:bg-elevated [&_pre]:p-3 [&_pre]:rounded-lg [&_pre]:overflow-x-auto [&_pre]:my-3',
              '[&_code]:bg-elevated [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-amber [&_code]:text-xs [&_code]:font-mono',
              '[&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_pre_code]:text-foreground',
              '[&_a]:text-cyan [&_a]:no-underline hover:[&_a]:underline',
              '[&_ul]:list-disc [&_ul]:pl-5 [&_ul]:my-3',
              '[&_ol]:list-decimal [&_ol]:pl-5 [&_ol]:my-3',
              '[&_li]:mb-1',
              '[&_strong]:text-foreground [&_strong]:font-semibold',
              '[&_h1]:text-xl [&_h1]:font-semibold [&_h1]:text-foreground [&_h1]:mb-3 [&_h1]:mt-4 first:[&_h1]:mt-0',
              '[&_h2]:text-lg [&_h2]:font-semibold [&_h2]:text-foreground [&_h2]:mb-2 [&_h2]:mt-4 first:[&_h2]:mt-0',
              '[&_h3]:text-base [&_h3]:font-semibold [&_h3]:text-foreground [&_h3]:mb-2 [&_h3]:mt-3 first:[&_h3]:mt-0',
              '[&_blockquote]:border-l-2 [&_blockquote]:border-amber/50 [&_blockquote]:pl-3 [&_blockquote]:italic [&_blockquote]:my-3',
              '[&_hr]:border-border [&_hr]:my-4'
            )}
          >
            <ReactMarkdown>{content}</ReactMarkdown>
          </div>
        ) : (
          <div className="text-sm text-muted-foreground/50 italic">
            Prompt will appear here as you chat...
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
