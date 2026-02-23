export interface ProjectConfigOverrides {
  defaultAgent?: string
  defaultModel?: string
}

export interface ProjectRecord {
  id: string
  path: string
  name: string
  configOverrides?: ProjectConfigOverrides
  isDefault: boolean
  createdAt: string
  lastAccessedAt: string
}

export interface ProjectListResponse {
  projects: ProjectRecord[]
}
