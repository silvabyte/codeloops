import { useRef, useEffect, useCallback } from 'react'
import { cn } from '@/lib/utils'
import { workTypeConfigs, type WorkType } from '@/lib/work-type-config'

export type { WorkType } from '@/lib/work-type-config'

interface WorkTypeSelectorProps {
  projectName: string
  onSelect: (type: WorkType) => void
  onChangeProject?: () => void
  isExiting?: boolean
}

export function WorkTypeSelector({ projectName, onSelect, onChangeProject, isExiting }: WorkTypeSelectorProps) {
  const navRef = useRef<HTMLElement>(null)
  const buttonRefs = useRef<(HTMLButtonElement | null)[]>([])

  // Number key shortcuts (1-5)
  useEffect(() => {
    if (isExiting) return

    const handleKeyDown = (e: KeyboardEvent) => {
      // Skip if user is typing in an input
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return

      const num = parseInt(e.key)
      if (num >= 1 && num <= workTypeConfigs.length) {
        e.preventDefault()
        onSelect(workTypeConfigs[num - 1].type)
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [onSelect, isExiting])

  // Roving tabindex keyboard navigation
  const handleNavKeyDown = useCallback((e: React.KeyboardEvent, index: number) => {
    let nextIndex: number | null = null

    switch (e.key) {
      case 'ArrowRight':
        e.preventDefault()
        nextIndex = (index + 1) % workTypeConfigs.length
        break
      case 'ArrowLeft':
        e.preventDefault()
        nextIndex = (index - 1 + workTypeConfigs.length) % workTypeConfigs.length
        break
      case 'Home':
        e.preventDefault()
        nextIndex = 0
        break
      case 'End':
        e.preventDefault()
        nextIndex = workTypeConfigs.length - 1
        break
      case 'Enter':
      case ' ':
        e.preventDefault()
        onSelect(workTypeConfigs[index].type)
        return
    }

    if (nextIndex !== null) {
      buttonRefs.current[nextIndex]?.focus()
    }
  }, [onSelect])

  return (
    <div className={cn(
      'flex flex-col items-center justify-center min-h-[60vh]',
      isExiting && 'work-type-exit'
    )}>
      {/* Ambient glow */}
      <div
        className="fixed top-0 left-1/2 -translate-x-1/2 w-[800px] h-[600px] pointer-events-none -z-10"
        style={{ background: 'radial-gradient(ellipse at center, rgba(245, 158, 11, 0.1) 0%, transparent 70%)' }}
      />

      {/* Card container */}
      <div className="relative px-12 py-10 rounded-xl border border-border bg-surface">
        {/* Header */}
        <div className="text-center space-y-3 mb-8">
          <h1 className="text-2xl font-semibold tracking-tight text-foreground">
            What are you building?
          </h1>
          <p className="text-sm text-muted-foreground">
            Select a work type to generate a structured prompt
          </p>
        </div>

        {/* Work type buttons */}
        <nav ref={navRef} className="flex items-center justify-center gap-3 mb-8" role="radiogroup" aria-label="Work type">
          {workTypeConfigs.map(({ type, label, description, icon: Icon }, index) => (
            <button
              key={type}
              ref={(el) => { buttonRefs.current[index] = el }}
              onClick={() => onSelect(type)}
              onKeyDown={(e) => handleNavKeyDown(e, index)}
              tabIndex={index === 0 ? 0 : -1}
              role="radio"
              aria-checked={false}
              className={cn(
                'work-type-card group relative px-5 py-3 rounded-lg transition-all duration-200',
                'bg-background border border-border',
                'hover:border-amber-dim hover:shadow-[0_8px_30px_rgba(245,158,11,0.15)]',
                'hover:-translate-y-0.5',
                'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-amber/50 focus-visible:border-amber-dim'
              )}
              style={{ animationDelay: `${index * 60}ms` }}
            >
              <Icon className="w-5 h-5 mx-auto mb-1.5 text-dim group-hover:text-amber group-focus-visible:text-amber transition-colors" />
              <span className="block text-sm font-medium text-foreground/90 group-hover:text-white transition-colors">
                {label}
              </span>
              <span className="block text-xs mt-0.5 text-dim">
                {description}
              </span>
            </button>
          ))}
        </nav>

        {/* Project breadcrumb */}
        <div className="flex items-center justify-center">
          <button
            onClick={onChangeProject}
            className="group inline-flex items-center gap-1 text-xs transition-all text-dim"
          >
            {/* Folder icon */}
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              className="opacity-50 group-hover:opacity-100 transition-opacity text-amber"
            >
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            </svg>
            {/* Path separator */}
            <span className="opacity-30">/</span>
            {/* Project name */}
            <span className="text-muted-foreground group-hover:underline underline-offset-2">
              {projectName}
            </span>
          </button>
        </div>
      </div>
    </div>
  )
}
