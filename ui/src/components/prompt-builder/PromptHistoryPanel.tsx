import { useState, useEffect, useCallback } from 'react'
import { cn, formatDate } from '@/lib/utils'
import type { PromptSummary, ListPromptsResponse } from '@/lib/prompt-session'
import { listPrompts, deletePrompt } from '@/lib/prompt-session'

interface PromptHistoryPanelProps {
  isOpen: boolean
  onClose: () => void
  onSelect: (id: string) => void
  currentProjectName?: string
}

function WorkTypeBadge({ workType }: { workType: string }) {
  const colors: Record<string, string> = {
    feature: 'bg-primary/20 text-primary',
    defect: 'bg-destructive/20 text-destructive',
    chore: 'bg-muted text-muted-foreground',
    research: 'bg-warning/20 text-warning',
    custom: 'bg-secondary text-secondary-foreground',
  }

  return (
    <span className={cn('px-2 py-0.5 rounded-full text-xs font-medium', colors[workType] || 'bg-muted text-muted-foreground')}>
      {workType}
    </span>
  )
}

export function PromptHistoryPanel({ isOpen, onClose, onSelect, currentProjectName }: PromptHistoryPanelProps) {
  const [prompts, setPrompts] = useState<PromptSummary[]>([])
  const [projects, setProjects] = useState<string[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Filters
  const [selectedProject, setSelectedProject] = useState<string>('')
  const [searchQuery, setSearchQuery] = useState('')
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null)

  const fetchPrompts = useCallback(async () => {
    setLoading(true)
    setError(null)

    try {
      const response: ListPromptsResponse = await listPrompts({
        projectName: selectedProject || undefined,
        search: searchQuery || undefined,
        limit: 50,
      })
      setPrompts(response.prompts)
      setProjects(response.projects)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load prompts')
    } finally {
      setLoading(false)
    }
  }, [selectedProject, searchQuery])

  useEffect(() => {
    if (isOpen) {
      fetchPrompts()
    }
  }, [isOpen, fetchPrompts])

  const handleDelete = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation()

    if (deleteConfirm !== id) {
      setDeleteConfirm(id)
      return
    }

    try {
      await deletePrompt(id)
      setPrompts(prev => prev.filter(p => p.id !== id))
      setDeleteConfirm(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete prompt')
    }
  }

  const handleSelect = (id: string) => {
    onSelect(id)
    onClose()
  }

  // Close delete confirm when clicking elsewhere
  useEffect(() => {
    const handleClickOutside = () => setDeleteConfirm(null)
    if (deleteConfirm) {
      document.addEventListener('click', handleClickOutside)
      return () => document.removeEventListener('click', handleClickOutside)
    }
  }, [deleteConfirm])

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-card border border-border rounded-lg shadow-lg w-full max-w-4xl max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border">
          <h2 className="text-lg font-medium">Prompt History</h2>
          <button
            onClick={onClose}
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            <span className="text-xl">&times;</span>
          </button>
        </div>

        {/* Filters */}
        <div className="flex gap-4 px-6 py-3 border-b border-border">
          <input
            type="text"
            placeholder="Search prompts..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="flex-1 px-3 py-1.5 rounded-md border border-border bg-secondary text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          />
          <select
            value={selectedProject}
            onChange={(e) => setSelectedProject(e.target.value)}
            className="px-3 py-1.5 rounded-md border border-border bg-secondary text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          >
            <option value="">All projects</option>
            {projects.map((project) => (
              <option key={project} value={project}>
                {project}
              </option>
            ))}
          </select>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto">
          {loading ? (
            <div className="space-y-2 p-4">
              {[...Array(5)].map((_, i) => (
                <div key={i} className="h-16 bg-secondary/50 rounded-lg animate-pulse" />
              ))}
            </div>
          ) : error ? (
            <div className="flex flex-col items-center justify-center py-12 text-destructive">
              <p>{error}</p>
              <button
                onClick={fetchPrompts}
                className="mt-4 text-sm text-muted-foreground hover:text-foreground"
              >
                Try again
              </button>
            </div>
          ) : prompts.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <p>No prompts yet</p>
              <p className="text-sm mt-1">Your saved prompts will appear here</p>
            </div>
          ) : (
            <div className="divide-y divide-border">
              {prompts.map((prompt) => (
                <div
                  key={prompt.id}
                  onClick={() => handleSelect(prompt.id)}
                  className={cn(
                    'px-6 py-3 hover:bg-secondary/30 cursor-pointer transition-colors group',
                    prompt.projectName === currentProjectName && 'bg-secondary/20'
                  )}
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-medium truncate">
                          {prompt.title || 'Untitled prompt'}
                        </span>
                        <WorkTypeBadge workType={prompt.workType} />
                      </div>
                      <div className="flex items-center gap-3 text-sm text-muted-foreground">
                        <span>{prompt.projectName}</span>
                        <span>&middot;</span>
                        <span>{formatDate(prompt.updatedAt)}</span>
                      </div>
                      {prompt.contentPreview && (
                        <p className="text-sm text-muted-foreground/70 mt-1 truncate">
                          {prompt.contentPreview}
                        </p>
                      )}
                    </div>
                    <button
                      onClick={(e) => handleDelete(prompt.id, e)}
                      className={cn(
                        'px-2 py-1 text-xs rounded transition-colors',
                        deleteConfirm === prompt.id
                          ? 'bg-destructive text-destructive-foreground'
                          : 'text-muted-foreground hover:text-destructive opacity-0 group-hover:opacity-100'
                      )}
                    >
                      {deleteConfirm === prompt.id ? 'Confirm?' : 'Delete'}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-3 border-t border-border text-sm text-muted-foreground">
          {prompts.length} prompt{prompts.length !== 1 ? 's' : ''}
        </div>
      </div>
    </div>
  )
}
