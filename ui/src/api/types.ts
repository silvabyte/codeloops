export interface SessionSummary {
  id: string
  timestamp: string
  prompt_preview: string
  working_dir: string
  project: string
  outcome: string | null
  iterations: number
  duration_secs: number | null
  confidence: number | null
  actor_agent: string
  critic_agent: string
}

export interface SessionStart {
  timestamp: string
  prompt: string
  working_dir: string
  actor_agent: string
  critic_agent: string
  actor_model: string | null
  critic_model: string | null
  max_iterations: number | null
}

export interface Iteration {
  iteration_number: number
  actor_output: string
  actor_stderr: string
  actor_exit_code: number
  actor_duration_secs: number
  git_diff: string
  git_files_changed: number
  critic_decision: string
  feedback: string | null
  timestamp: string
}

export interface SessionEnd {
  outcome: string
  iterations: number
  summary: string | null
  confidence: number | null
  duration_secs: number
  timestamp: string
}

export interface Session {
  id: string
  start: SessionStart
  iterations: Iteration[]
  end: SessionEnd | null
}

export interface DayCount {
  date: string
  count: number
}

export interface ProjectStats {
  project: string
  total: number
  success_rate: number
}

export interface SessionStats {
  total_sessions: number
  success_rate: number
  avg_iterations: number
  avg_duration_secs: number
  sessions_over_time: DayCount[]
  by_project: ProjectStats[]
}

export interface SessionFilter {
  outcome?: string
  after?: string
  before?: string
  search?: string
  project?: string
}

export type SessionEventType = 'session_created' | 'session_updated' | 'session_completed'

export interface SessionEvent {
  event: SessionEventType
  id: string
  iteration?: number
  outcome?: string
}
