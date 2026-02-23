import { createContext, useContext, type ReactNode } from 'react'
import { useParams, Navigate } from 'react-router-dom'
import { useCallback, useEffect, useState } from 'react'
import { fetchProjects } from '@/api/client'
import type { ProjectRecord } from '@/types/project'

// ---------------------------------------------------------------------------
// Context — provides the current projectId from URL params
// ---------------------------------------------------------------------------

const ProjectContext = createContext<string | null>(null)

export function ProjectProvider({ children }: { children: ReactNode }) {
  const { projectId } = useParams<{ projectId: string }>()
  return (
    <ProjectContext.Provider value={projectId ?? null}>
      {children}
    </ProjectContext.Provider>
  )
}

/**
 * Returns the current project ID from URL params.
 * Must be used within a `<ProjectProvider>`.
 */
export function useCurrentProject(): string {
  const projectId = useContext(ProjectContext)
  if (!projectId) throw new Error('useCurrentProject must be used within a ProjectProvider')
  return projectId
}

// ---------------------------------------------------------------------------
// Redirect — sends `/` to the default project
// ---------------------------------------------------------------------------

export function ProjectRedirect() {
  const [defaultId, setDefaultId] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    async function resolve() {
      try {
        const { projects } = await fetchProjects()
        const def = projects.find((p) => p.isDefault) ?? projects[0]
        if (def) {
          setDefaultId(def.id)
        } else {
          setError('No projects registered. Run: codeloops project add .')
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to load projects')
      } finally {
        setLoading(false)
      }
    }
    resolve()
  }, [])

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <span className="text-sm text-muted-foreground animate-pulse">Loading...</span>
      </div>
    )
  }

  if (error || !defaultId) {
    return (
      <div className="flex flex-col items-center justify-center min-h-screen gap-4">
        <p className="text-sm text-destructive">{error}</p>
      </div>
    )
  }

  return <Navigate to={`/projects/${defaultId}`} replace />
}

// ---------------------------------------------------------------------------
// Projects list hook — for the project management page
// ---------------------------------------------------------------------------

export function useProjects() {
  const [projects, setProjects] = useState<ProjectRecord[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setLoading(true)
      const { projects } = await fetchProjects()
      setProjects(projects)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load projects')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { load() }, [load])

  return { projects, loading, error, reload: load }
}
