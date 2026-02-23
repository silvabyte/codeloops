import { Sparkles, Bug, ShieldAlert, Wrench, Puzzle, type LucideIcon } from 'lucide-react'

export type WorkType = 'feature' | 'defect' | 'risk' | 'debt' | 'custom'

export interface WorkTypeConfig {
  type: WorkType
  label: string
  description: string
  icon: LucideIcon
}

export const workTypeConfigs: WorkTypeConfig[] = [
  { type: 'feature', label: 'Feature', description: 'New capability', icon: Sparkles },
  { type: 'defect', label: 'Defect', description: 'Fix a bug', icon: Bug },
  { type: 'risk', label: 'Risk', description: 'Address concern', icon: ShieldAlert },
  { type: 'debt', label: 'Debt', description: 'Improve code', icon: Wrench },
  { type: 'custom', label: 'Custom', description: 'Something else', icon: Puzzle },
]

/** Lookup map for getting config by work type */
export const workTypeMap = Object.fromEntries(
  workTypeConfigs.map(c => [c.type, c])
) as Record<WorkType, WorkTypeConfig>
