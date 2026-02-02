import { cn } from '@/lib/utils'

export type WorkType = 'feature' | 'defect' | 'risk' | 'debt' | 'custom'

interface WorkTypeSelectorProps {
  projectName: string
  onSelect: (type: WorkType) => void
  onChangeProject?: () => void
}

const workTypes: { type: WorkType; label: string; description: string }[] = [
  { type: 'feature', label: 'Feature', description: 'New capability' },
  { type: 'defect', label: 'Defect', description: 'Fix a bug' },
  { type: 'risk', label: 'Risk', description: 'Address concern' },
  { type: 'debt', label: 'Debt', description: 'Improve code' },
  { type: 'custom', label: 'Custom', description: 'Something else' },
]

export function WorkTypeSelector({ projectName, onSelect, onChangeProject }: WorkTypeSelectorProps) {
  return (
    <div className="flex flex-col items-center justify-center min-h-[60vh]">
      {/* Ambient glow */}
      <div
        className="fixed top-0 left-1/2 -translate-x-1/2 w-[800px] h-[600px] pointer-events-none -z-10"
        style={{ background: 'radial-gradient(ellipse at center, rgba(245, 158, 11, 0.1) 0%, transparent 70%)' }}
      />

      {/* Card container */}
      <div
        className="relative px-12 py-10 rounded-xl border"
        style={{ backgroundColor: 'var(--color-surface)', borderColor: 'var(--color-border)' }}
      >
        {/* Header */}
        <div className="text-center space-y-3 mb-8">
          <h1
            className="text-2xl font-semibold tracking-tight"
            style={{ color: 'var(--color-foreground)' }}
          >
            What are you building?
          </h1>
          <p className="text-sm" style={{ color: 'var(--color-muted-foreground)' }}>
            Select a work type to generate a structured prompt
          </p>
        </div>

        {/* Work type buttons */}
        <nav className="flex items-center justify-center gap-3 mb-8">
          {workTypes.map(({ type, label, description }) => (
            <button
              key={type}
              onClick={() => onSelect(type)}
              className={cn(
                'group relative px-5 py-3 rounded-lg transition-all duration-200',
                'hover:-translate-y-0.5'
              )}
              style={{
                backgroundColor: 'var(--color-background)',
                border: '1px solid var(--color-border)',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.borderColor = 'var(--color-amber-dim)'
                e.currentTarget.style.boxShadow = '0 8px 30px rgba(245, 158, 11, 0.2)'
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.borderColor = 'var(--color-border)'
                e.currentTarget.style.boxShadow = 'none'
              }}
            >
              <span
                className="block text-sm font-medium group-hover:text-white"
                style={{ color: 'var(--color-foreground)', opacity: 0.9 }}
              >
                {label}
              </span>
              <span
                className="block text-xs mt-0.5"
                style={{ color: 'var(--color-dim)' }}
              >
                {description}
              </span>
            </button>
          ))}
        </nav>

        {/* Project indicator */}
        <div
          className="flex items-center justify-center gap-2 text-xs"
          style={{ color: 'var(--color-dim)' }}
        >
          <span>Working in</span>
          <button
            onClick={onChangeProject}
            className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full transition-all hover:opacity-100"
            style={{
              backgroundColor: 'var(--color-elevated)',
              border: '1px solid var(--color-border)',
              color: 'var(--color-muted-foreground)',
            }}
          >
            <span
              className="w-1.5 h-1.5 rounded-full"
              style={{ backgroundColor: 'var(--color-amber)' }}
            />
            {projectName}
          </button>
        </div>
      </div>
    </div>
  )
}
