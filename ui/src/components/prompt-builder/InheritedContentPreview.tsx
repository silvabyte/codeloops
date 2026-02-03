import { useState, useEffect } from 'react'
import { cn } from '@/lib/utils'
import type { ResolvedPromptResponse } from '@/lib/prompt-session'
import { getResolvedPrompt } from '@/lib/prompt-session'
import ReactMarkdown from 'react-markdown'
import { ChevronDown, ChevronRight } from 'lucide-react'

interface InheritedContentPreviewProps {
  promptId: string
  parentIds: string[]
}

export function InheritedContentPreview({ promptId, parentIds }: InheritedContentPreviewProps) {
  const [resolved, setResolved] = useState<ResolvedPromptResponse | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [isExpanded, setIsExpanded] = useState(false)

  useEffect(() => {
    if (parentIds.length === 0) {
      setResolved(null)
      return
    }

    const fetchResolved = async () => {
      setLoading(true)
      setError(null)
      try {
        const data = await getResolvedPrompt(promptId)
        setResolved(data)
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to load inherited content')
      } finally {
        setLoading(false)
      }
    }

    fetchResolved()
  }, [promptId, parentIds])

  if (parentIds.length === 0) {
    return null
  }

  // Count inherited prompts (exclude self from chain)
  const inheritedCount = resolved ? resolved.chain.length - 1 : parentIds.length

  return (
    <div className="border-b border-border">
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className={cn(
          'w-full flex items-center gap-2 px-4 py-2 text-sm',
          'text-muted-foreground hover:text-foreground transition-colors'
        )}
      >
        {isExpanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
        <span>
          Inherits from {inheritedCount} prompt{inheritedCount !== 1 ? 's' : ''}
        </span>
      </button>

      {isExpanded && (
        <div className="px-4 pb-4">
          {loading ? (
            <div className="space-y-2 animate-pulse">
              <div className="h-4 w-3/4 bg-border/50 rounded" />
              <div className="h-4 w-1/2 bg-border/30 rounded" />
            </div>
          ) : error ? (
            <p className="text-sm text-destructive">{error}</p>
          ) : resolved && resolved.chain.length > 1 ? (
            <div className="space-y-3">
              {/* Show chain */}
              <div className="flex flex-wrap gap-1 mb-2">
                {resolved.chain.slice(0, -1).map((item, index) => (
                  <span
                    key={item.id}
                    className="inline-flex items-center text-xs text-amber-500"
                  >
                    {index > 0 && <span className="mx-1 text-muted-foreground">&rarr;</span>}
                    {item.title || 'Untitled'}
                  </span>
                ))}
              </div>

              {/* Show inherited content (excluding current prompt's content) */}
              {resolved.resolvedContent && (
                <div
                  className={cn(
                    'text-sm leading-relaxed max-w-none text-foreground/70',
                    'bg-surface/50 rounded-lg p-3 max-h-48 overflow-y-auto',
                    // Markdown element styling (same as PreviewPanel)
                    '[&_p]:mb-2 [&_p:last-child]:mb-0',
                    '[&_pre]:bg-elevated [&_pre]:p-2 [&_pre]:rounded-lg [&_pre]:overflow-x-auto [&_pre]:my-2',
                    '[&_code]:bg-elevated [&_code]:px-1 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-amber [&_code]:text-xs [&_code]:font-mono',
                    '[&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_pre_code]:text-foreground',
                    '[&_ul]:list-disc [&_ul]:pl-4 [&_ul]:my-2',
                    '[&_ol]:list-decimal [&_ol]:pl-4 [&_ol]:my-2',
                    '[&_li]:mb-0.5',
                    '[&_strong]:text-foreground/90 [&_strong]:font-semibold',
                    '[&_h1]:text-base [&_h1]:font-semibold [&_h1]:text-foreground/90 [&_h1]:mb-2 [&_h1]:mt-3 first:[&_h1]:mt-0',
                    '[&_h2]:text-sm [&_h2]:font-semibold [&_h2]:text-foreground/90 [&_h2]:mb-1 [&_h2]:mt-3 first:[&_h2]:mt-0',
                    '[&_h3]:text-sm [&_h3]:font-semibold [&_h3]:text-foreground/90 [&_h3]:mb-1 [&_h3]:mt-2 first:[&_h3]:mt-0'
                  )}
                >
                  <ReactMarkdown>{resolved.resolvedContent}</ReactMarkdown>
                </div>
              )}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">No inherited content</p>
          )}
        </div>
      )}
    </div>
  )
}
