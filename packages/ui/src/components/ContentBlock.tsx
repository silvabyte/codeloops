import ReactMarkdown from 'react-markdown'
import { cn } from '@/lib/utils'
import { CopyButton } from './CopyButton'

/**
 * ContentBlock - Labeled content container with copy functionality.
 * Used for displaying actor output, critic feedback, prompts, etc.
 *
 * @param label - Header label (e.g., "ACTOR", "CRITIC", "PROMPT")
 * @param content - The text content to display
 * @param variant - Styling variant: 'actor' (cyan border), 'critic' (amber border), or 'default'
 * @param markdown - If true, render content as markdown instead of preformatted text
 */
interface ContentBlockProps {
  label: string
  content: string
  variant?: 'actor' | 'critic' | 'default'
  className?: string
  markdown?: boolean
}

const markdownStyles = cn(
  'text-sm leading-relaxed max-w-none',
  // Paragraphs
  '[&_p]:mb-3 [&_p:last-child]:mb-0',
  // Code blocks
  '[&_pre]:bg-elevated [&_pre]:p-3 [&_pre]:rounded-lg [&_pre]:overflow-x-auto [&_pre]:my-3',
  '[&_code]:bg-elevated [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-amber [&_code]:text-xs',
  '[&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_pre_code]:text-foreground',
  // Links
  '[&_a]:text-cyan [&_a]:no-underline hover:[&_a]:underline',
  // Lists
  '[&_ul]:list-disc [&_ul]:pl-5 [&_ul]:my-3',
  '[&_ol]:list-decimal [&_ol]:pl-5 [&_ol]:my-3',
  '[&_li]:mb-1',
  // Emphasis
  '[&_strong]:text-foreground [&_strong]:font-semibold',
  '[&_em]:italic',
  // Headings
  '[&_h1]:text-lg [&_h1]:font-semibold [&_h1]:text-foreground [&_h1]:mb-3 [&_h1]:mt-4 [&_h1:first-child]:mt-0',
  '[&_h2]:text-base [&_h2]:font-semibold [&_h2]:text-foreground [&_h2]:mb-2 [&_h2]:mt-4 [&_h2:first-child]:mt-0',
  '[&_h3]:text-sm [&_h3]:font-semibold [&_h3]:text-foreground [&_h3]:mb-2 [&_h3]:mt-3 [&_h3:first-child]:mt-0',
  // Blockquote
  '[&_blockquote]:border-l-2 [&_blockquote]:border-amber/50 [&_blockquote]:pl-3 [&_blockquote]:italic [&_blockquote]:my-3',
  // Horizontal rule
  '[&_hr]:border-border [&_hr]:my-4'
)

export function ContentBlock({ label, content, variant = 'default', className, markdown = false }: ContentBlockProps) {
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
        {markdown ? (
          <div className={markdownStyles}>
            <ReactMarkdown>{content}</ReactMarkdown>
          </div>
        ) : (
          <pre className="text-sm whitespace-pre-wrap font-mono leading-relaxed">
            {content}
          </pre>
        )}
      </div>
    </div>
  )
}
