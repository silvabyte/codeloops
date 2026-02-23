import { useState } from 'react'
import { cn } from '@/lib/utils'
import type { SessionFilter } from '@/api/types'

interface SessionFiltersProps {
  onFilterChange: (filter: SessionFilter) => void
}

const OUTCOMES = [
  { value: 'success', label: 'success' },
  { value: 'failed', label: 'failed' },
  { value: 'interrupted', label: 'interrupted' },
  { value: 'max_iterations_reached', label: 'max iter' },
]

export function SessionFilters({ onFilterChange }: SessionFiltersProps) {
  const [selectedOutcomes, setSelectedOutcomes] = useState<Set<string>>(new Set())
  const [search, setSearch] = useState('')

  const applyFilters = (updates: Partial<{ outcomes: Set<string>; search: string }>) => {
    const newOutcomes = updates.outcomes ?? selectedOutcomes
    const newSearch = updates.search ?? search

    // If multiple outcomes selected, we pass comma-separated values
    // But current API only supports single outcome, so we pass first one
    const outcome = newOutcomes.size === 1 ? Array.from(newOutcomes)[0] : undefined

    onFilterChange({
      outcome,
      search: newSearch || undefined,
    })
  }

  const toggleOutcome = (value: string) => {
    const newOutcomes = new Set(selectedOutcomes)
    if (newOutcomes.has(value)) {
      newOutcomes.delete(value)
    } else {
      newOutcomes.clear() // Only one at a time for now
      newOutcomes.add(value)
    }
    setSelectedOutcomes(newOutcomes)
    applyFilters({ outcomes: newOutcomes })
  }

  return (
    <div className="flex flex-wrap gap-3 items-center">
      <input
        type="text"
        placeholder="Search prompts or projects..."
        value={search}
        onChange={(e) => {
          setSearch(e.target.value)
          applyFilters({ search: e.target.value })
        }}
        className="px-3 py-1.5 rounded-md border border-border bg-secondary text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring flex-1 min-w-[200px] max-w-sm"
      />
      <div className="flex gap-1.5">
        {OUTCOMES.map((o) => {
          const isActive = selectedOutcomes.has(o.value)
          return (
            <button
              key={o.value}
              onClick={() => toggleOutcome(o.value)}
              className={cn(
                'px-2.5 py-1 rounded-md text-xs font-medium transition-colors',
                isActive
                  ? 'bg-amber-500/15 text-amber-500 border border-amber-500/30'
                  : 'text-muted-foreground hover:text-foreground hover:bg-secondary border border-transparent'
              )}
            >
              {o.label}
            </button>
          )
        })}
      </div>
    </div>
  )
}
