export interface SessionSummary {
  id: string
  timestamp: string
  promptPreview: string
  workingDir: string
  project: string
  outcome: string | null
  iterations: number
  durationSecs: number | null
  confidence: number | null
  actorAgent: string
  criticAgent: string
}

export interface SessionStart {
  timestamp: string
  prompt: string
  workingDir: string
  actorAgent: string
  criticAgent: string
  actorModel: string | null
  criticModel: string | null
  maxIterations: number | null
}

export interface Iteration {
  iterationNumber: number
  actorOutput: string
  actorStderr: string
  actorExitCode: number
  actorDurationSecs: number
  gitDiff: string
  gitFilesChanged: number
  criticDecision: string
  feedback: string | null
  timestamp: string
}

export interface SessionEnd {
  outcome: string
  iterations: number
  summary: string | null
  confidence: number | null
  durationSecs: number
  timestamp: string
}

export interface Session {
  id: string
  prompt: string
  workingDir: string
  actorAgent: string
  criticAgent: string
  actorModel: string | null
  criticModel: string | null
  maxIterations: number | null
  outcome: string | null
  iterationCount: number | null
  summary: string | null
  confidence: number | null
  durationSecs: number | null
  startedAt: string
  endedAt: string | null
  iterations: Iteration[]
}

export interface DayCount {
  date: string
  count: number
}

export interface ProjectStats {
  project: string
  total: number
  successRate: number
}

export interface SessionStats {
  totalSessions: number
  successRate: number
  avgIterations: number
  avgDurationSecs: number
  sessionsOverTime: DayCount[]
  byProject: ProjectStats[]
}

export interface AgenticMetrics {
  // Session metrics
  totalSessions: number
  successfulSessions: number
  successRate: number
  firstTrySuccessRate: number
  avgIterationsToSuccess: number
  avgCycleTimeSecs: number
  wasteRate: number

  // Critic metrics
  totalIterations: number
  criticApprovalRate: number
  avgFeedbackLength: number
  improvementRate: number

  // Breakdowns
  sessionsOverTime: DayCount[]
  byProject: ProjectStats[]
}

export interface SessionFilter {
  outcome?: string
  after?: string
  before?: string
  search?: string
  project?: string
}
