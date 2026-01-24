import { useState } from 'react'
import type { SessionFilter } from '@/api/types'

interface SessionFiltersProps {
  onFilterChange: (filter: SessionFilter) => void
}

const OUTCOMES = ['success', 'failed', 'interrupted', 'max_iterations_reached']

export function SessionFilters({ onFilterChange }: SessionFiltersProps) {
  const [outcome, setOutcome] = useState<string>('')
  const [search, setSearch] = useState('')
  const [project, setProject] = useState('')

  const applyFilters = (updates: Partial<{ outcome: string; search: string; project: string }>) => {
    const newOutcome = updates.outcome ?? outcome
    const newSearch = updates.search ?? search
    const newProject = updates.project ?? project

    onFilterChange({
      outcome: newOutcome || undefined,
      search: newSearch || undefined,
      project: newProject || undefined,
    })
  }

  return (
    <div className="flex gap-3 items-center">
      <input
        type="text"
        placeholder="Search prompts..."
        value={search}
        onChange={(e) => {
          setSearch(e.target.value)
          applyFilters({ search: e.target.value })
        }}
        className="px-3 py-1.5 rounded-md border border-border bg-secondary text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring w-64"
      />
      <select
        value={outcome}
        onChange={(e) => {
          setOutcome(e.target.value)
          applyFilters({ outcome: e.target.value })
        }}
        className="px-3 py-1.5 rounded-md border border-border bg-secondary text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-ring"
      >
        <option value="">All outcomes</option>
        {OUTCOMES.map((o) => (
          <option key={o} value={o}>{o}</option>
        ))}
      </select>
      <input
        type="text"
        placeholder="Project..."
        value={project}
        onChange={(e) => {
          setProject(e.target.value)
          applyFilters({ project: e.target.value })
        }}
        className="px-3 py-1.5 rounded-md border border-border bg-secondary text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring w-40"
      />
    </div>
  )
}
