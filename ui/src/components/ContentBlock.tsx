import { cn } from '@/lib/utils'
import { CopyButton } from './CopyButton'

/**
 * ContentBlock - Labeled content container with copy functionality.
 * Used for displaying actor output, critic feedback, prompts, etc.
 *
 * @param label - Header label (e.g., "ACTOR", "CRITIC", "PROMPT")
 * @param content - The text content to display
 * @param variant - Styling variant: 'actor' (cyan border), 'critic' (amber border), or 'default'
 */
interface ContentBlockProps {
  label: string
  content: string
  variant?: 'actor' | 'critic' | 'default'
  className?: string
}

export function ContentBlock({ label, content, variant = 'default', className }: ContentBlockProps) {
  return (
    <div
      className={cn(
        'bg-surface rounded-lg border border-border overflow-hidden',
        variant === 'actor' && 'border-l-2 border-l-cyan-dim',
        variant === 'critic' && 'border-l-2 border-l-amber-dim',
        className
      )}
    >
      <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-elevated/30">
        <span className="text-xs text-muted-foreground uppercase tracking-wider font-medium">
          {label}
        </span>
        <CopyButton content={content} />
      </div>
      <div className="p-4">
        <pre className="text-sm whitespace-pre-wrap font-mono leading-relaxed">
          {content}
        </pre>
      </div>
    </div>
  )
}
