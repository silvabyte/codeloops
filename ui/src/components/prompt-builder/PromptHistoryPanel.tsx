import { useState, useEffect, useCallback, useRef } from 'react'
import { cn, formatDate } from '@/lib/utils'
import type { PromptSummary, ListPromptsResponse } from '@/lib/prompt-session'
import { listPrompts, deletePrompt } from '@/lib/prompt-session'
import { X } from 'lucide-react'

interface PromptHistoryPanelProps {
  isOpen: boolean
  onClose: () => void
  onSelect: (id: string) => void
  currentProjectName?: string
}

export function PromptHistoryPanel({ isOpen, onClose, onSelect, currentProjectName }: PromptHistoryPanelProps) {
  const [prompts, setPrompts] = useState<PromptSummary[]>([])
  const [projects, setProjects] = useState<string[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [isVisible, setIsVisible] = useState(false)

  // Filters
  const [selectedProject, setSelectedProject] = useState<string>('')
  const [searchQuery, setSearchQuery] = useState('')
  const [debouncedSearch, setDebouncedSearch] = useState('')
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null)
  const [focusedIndex, setFocusedIndex] = useState(-1)

  const listRef = useRef<HTMLDivElement>(null)
  const searchRef = useRef<HTMLInputElement>(null)

  // Debounce search input
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearch(searchQuery), 250)
    return () => clearTimeout(timer)
  }, [searchQuery])

  const fetchPrompts = useCallback(async () => {
    setLoading(true)
    setError(null)

    try {
      const response: ListPromptsResponse = await listPrompts({
        projectName: selectedProject || undefined,
        search: debouncedSearch || undefined,
        limit: 50,
      })
      setPrompts(response.prompts)
      setProjects(response.projects)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load prompts')
    } finally {
      setLoading(false)
    }
  }, [selectedProject, debouncedSearch])

  useEffect(() => {
    if (isOpen) {
      fetchPrompts()
      // Trigger entrance animation
      requestAnimationFrame(() => setIsVisible(true))
      // Focus search on open
      setTimeout(() => searchRef.current?.focus(), 100)
    } else {
      setIsVisible(false)
      setFocusedIndex(-1)
    }
  }, [isOpen, fetchPrompts])

  // Keyboard navigation
  useEffect(() => {
    if (!isOpen) return

    const handleKeyDown = (e: KeyboardEvent) => {
      switch (e.key) {
        case 'Escape':
          onClose()
          break
        case 'ArrowDown':
          e.preventDefault()
          setFocusedIndex(prev => Math.min(prev + 1, prompts.length - 1))
          break
        case 'ArrowUp':
          e.preventDefault()
          setFocusedIndex(prev => Math.max(prev - 1, -1))
          break
        case 'Enter':
          if (focusedIndex >= 0 && focusedIndex < prompts.length) {
            handleSelect(prompts[focusedIndex].id)
          }
          break
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [isOpen, focusedIndex, prompts, onClose])

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
    <div
      className={cn(
        "fixed inset-0 z-50 flex items-center justify-center transition-opacity duration-150",
        "bg-background/90",
        isVisible ? "opacity-100" : "opacity-0"
      )}
      onClick={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        className={cn(
          "bg-surface rounded-2xl w-full max-w-xl max-h-[70vh] flex flex-col overflow-hidden",
          "shadow-[0_0_0_1px_rgba(255,255,255,0.05),0_25px_50px_-12px_rgba(0,0,0,0.5)]",
          "transition-all duration-150",
          isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-2"
        )}
      >
        {/* Header + Search */}
        <div className="p-6 pb-0">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xl font-semibold tracking-tight">History</h2>
            <button
              onClick={onClose}
              className="p-1 -mr-1 text-dim hover:text-foreground transition-colors"
              aria-label="Close"
            >
              <X className="w-5 h-5" />
            </button>
          </div>

          <div className="flex gap-3">
            <input
              ref={searchRef}
              type="text"
              placeholder="Search..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="flex-1 px-0 py-2 bg-transparent text-sm placeholder:text-dim border-b border-border focus:border-foreground focus:outline-none transition-colors"
            />
            {projects.length > 1 && (
              <select
                value={selectedProject}
                onChange={(e) => setSelectedProject(e.target.value)}
                className="px-0 py-2 bg-transparent text-sm text-dim border-b border-border focus:border-foreground focus:outline-none transition-colors cursor-pointer"
              >
                <option value="">All</option>
                {projects.map((project) => (
                  <option key={project} value={project}>{project}</option>
                ))}
              </select>
            )}
          </div>
        </div>

        {/* Content */}
        <div ref={listRef} className="flex-1 overflow-auto px-6 py-6">
          {loading ? (
            <div className="space-y-6">
              {[...Array(4)].map((_, i) => (
                <div key={i} className="space-y-2">
                  <div className="h-4 w-48 bg-border/50 rounded animate-pulse" />
                  <div className="h-3 w-32 bg-border/30 rounded animate-pulse" />
                </div>
              ))}
            </div>
          ) : error ? (
            <div className="text-center py-12">
              <p className="text-destructive mb-4">{error}</p>
              <button onClick={fetchPrompts} className="text-sm text-dim hover:text-foreground transition-colors">
                Try again
              </button>
            </div>
          ) : prompts.length === 0 ? (
            <div className="text-center py-12 text-dim">
              <p>No prompts yet</p>
            </div>
          ) : (
            <div className="space-y-1">
              {prompts.map((prompt, index) => (
                <div
                  key={prompt.id}
                  onClick={() => handleSelect(prompt.id)}
                  onMouseEnter={() => setFocusedIndex(index)}
                  className={cn(
                    'group -mx-3 px-3 py-3 rounded-lg cursor-pointer transition-colors',
                    focusedIndex === index ? 'bg-hover' : 'hover:bg-hover/50'
                  )}
                >
                  <div className="flex items-baseline justify-between gap-4 mb-1">
                    <span className="font-medium truncate">
                      {prompt.title || 'Untitled'}
                    </span>
                    <span className="text-xs text-dim shrink-0">
                      {formatDate(prompt.updatedAt)}
                    </span>
                  </div>

                  <div className="flex items-center justify-between gap-4">
                    <span className="text-sm text-dim">
                      {prompt.projectName}
                    </span>
                    <button
                      onClick={(e) => handleDelete(prompt.id, e)}
                      className={cn(
                        'text-xs transition-all',
                        deleteConfirm === prompt.id
                          ? 'text-destructive'
                          : 'text-dim/0 group-hover:text-dim hover:text-destructive'
                      )}
                    >
                      {deleteConfirm === prompt.id ? 'confirm?' : 'delete'}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
