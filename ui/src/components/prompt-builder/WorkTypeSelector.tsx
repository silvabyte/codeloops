import { cn } from '@/lib/utils'

export type WorkType = 'feature' | 'defect' | 'risk' | 'debt' | 'custom'

interface WorkTypeSelectorProps {
  projectName: string
  onSelect: (type: WorkType) => void
  onChangeProject?: () => void
}

const workTypes: { type: WorkType; label: string }[] = [
  { type: 'feature', label: 'Feature' },
  { type: 'defect', label: 'Defect' },
  { type: 'risk', label: 'Risk' },
  { type: 'debt', label: 'Debt' },
  { type: 'custom', label: 'Custom' },
]

export function WorkTypeSelector({ projectName, onSelect, onChangeProject }: WorkTypeSelectorProps) {
  return (
    <div className="flex flex-col items-center justify-center min-h-[60vh] space-y-12">
      <div className="text-center space-y-2">
        <h1 className="text-xl text-muted-foreground font-normal">
          What are you building?
        </h1>
      </div>

      <nav className="flex items-center gap-8">
        {workTypes.map(({ type, label }) => (
          <button
            key={type}
            onClick={() => onSelect(type)}
            className={cn(
              'text-lg text-muted-foreground transition-all duration-200',
              'hover:text-foreground',
              'border-b border-transparent hover:border-foreground',
              'pb-1'
            )}
          >
            {label}
          </button>
        ))}
      </nav>

      <button
        onClick={onChangeProject}
        className="text-sm text-muted-foreground/60 hover:text-muted-foreground transition-colors"
      >
        {projectName}
      </button>
    </div>
  )
}
