import { useState, useEffect, useCallback } from 'react'
import ReactMarkdown from 'react-markdown'
import { cn } from '@/lib/utils'
import { markdownPreview } from '@/lib/markdown-styles'
import { FileText, Save, Copy, Download, ChevronDown, ChevronRight, Check, Puzzle } from 'lucide-react'
import { ParentPromptChips } from './ParentPromptChips'
import { ParentPromptSelector } from './ParentPromptSelector'
import { InheritedContentPreview } from './InheritedContentPreview'
import { fetchSkills } from '@/lib/prompt-session'
import type { SkillInfo } from '@/lib/prompt-session'

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
  enabledSkills?: string[]
  onToggleSkill?: (skillId: string) => void
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
  enabledSkills = [],
  onToggleSkill,
}: PreviewPanelProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [showParentSelector, setShowParentSelector] = useState(false)
  const [skillsExpanded, setSkillsExpanded] = useState(false)
  const [skills, setSkills] = useState<SkillInfo[]>([])
  const [skillsLoading, setSkillsLoading] = useState(true)

  // Fetch available skills on mount
  useEffect(() => {
    let cancelled = false
    async function load() {
      setSkillsLoading(true)
      const discovered = await fetchSkills()
      if (!cancelled) {
        setSkills(discovered)
        setSkillsLoading(false)
      }
    }
    load()
    return () => { cancelled = true }
  }, [])

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
  const enabledCount = enabledSkills.length

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

      {/* Skills Section */}
      {onToggleSkill && (
        <div className="border-b border-border">
          <button
            onClick={() => setSkillsExpanded(!skillsExpanded)}
            className="flex items-center gap-1.5 w-full px-4 py-2 text-left hover:bg-hover/50 transition-colors"
          >
            {skillsExpanded
              ? <ChevronDown className="w-3.5 h-3.5 text-dim" />
              : <ChevronRight className="w-3.5 h-3.5 text-dim" />}
            <span className="text-xs text-muted-foreground font-medium">Skills</span>
            {enabledCount > 0 && (
              <span className="text-[11px] text-amber bg-amber/10 px-1.5 py-0.5 rounded-full ml-auto">
                {enabledCount} active
              </span>
            )}
          </button>

          {skillsExpanded && (
            <div className="px-2 pb-2">
              {skillsLoading ? (
                <div className="space-y-2 px-2">
                  {[1, 2, 3].map((i) => (
                    <div key={i} className="animate-pulse flex items-center gap-3 py-2">
                      <div className="flex-1 space-y-1">
                        <div className="h-3 w-24 bg-border/50 rounded" />
                        <div className="h-2.5 w-40 bg-border/30 rounded" />
                      </div>
                      <div className="w-4 h-4 rounded-full bg-border/30" />
                    </div>
                  ))}
                </div>
              ) : skills.length === 0 ? (
                <div className="flex items-center gap-2 px-3 py-3">
                  <Puzzle className="w-3.5 h-3.5 text-dim" />
                  <span className="text-xs text-dim italic">
                    No skills found in ~/.agents/skills or ~/.claude/skills
                  </span>
                </div>
              ) : (
                <div className="space-y-0.5">
                  {skills.map((skill) => {
                    const isActive = enabledSkills.includes(skill.id)
                    return (
                      <button
                        key={skill.id}
                        onClick={() => onToggleSkill(skill.id)}
                        className={cn(
                          'flex items-center justify-between w-full px-3 py-2 rounded-md',
                          'text-left transition-colors group',
                          isActive
                            ? 'bg-amber/5 border border-amber/10 hover:bg-amber/10'
                            : 'hover:bg-hover border border-transparent'
                        )}
                      >
                        <div className="flex-1 min-w-0">
                          <div className={cn(
                            'text-xs transition-colors',
                            isActive
                              ? 'text-amber font-medium'
                              : 'text-muted-foreground group-hover:text-foreground'
                          )}>
                            {skill.name}
                          </div>
                          <div className={cn(
                            'text-[11px] truncate',
                            isActive ? 'text-amber/60' : 'text-dim'
                          )}>
                            {skill.description}
                          </div>
                        </div>
                        {isActive ? (
                          <div className="w-4 h-4 rounded-full bg-amber ml-3 flex-shrink-0 flex items-center justify-center">
                            <Check className="w-2.5 h-2.5 text-background" />
                          </div>
                        ) : (
                          <div className="w-4 h-4 rounded-full border border-border ml-3 flex-shrink-0 group-hover:border-muted-foreground transition-colors" />
                        )}
                      </button>
                    )
                  })}
                </div>
              )}
            </div>
          )}
        </div>
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
