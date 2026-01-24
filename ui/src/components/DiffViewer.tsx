interface DiffViewerProps {
  diff: string
}

export function DiffViewer({ diff }: DiffViewerProps) {
  if (!diff) {
    return <div className="text-muted-foreground text-sm">No diffs available.</div>
  }

  const lines = diff.split('\n')

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      <div className="text-xs text-muted-foreground uppercase tracking-wider px-4 py-2 bg-secondary/50 border-b border-border">
        Cumulative Diff
      </div>
      <pre className="p-4 text-xs overflow-x-auto max-h-[600px] overflow-y-auto bg-card">
        {lines.map((line, i) => {
          let className = ''
          if (line.startsWith('+') && !line.startsWith('+++')) {
            className = 'text-success bg-success/10'
          } else if (line.startsWith('-') && !line.startsWith('---')) {
            className = 'text-destructive bg-destructive/10'
          } else if (line.startsWith('@@')) {
            className = 'text-primary'
          } else if (line.startsWith('diff ') || line.startsWith('index ')) {
            className = 'text-muted-foreground font-bold'
          }

          return (
            <div key={i} className={className}>
              {line}
            </div>
          )
        })}
      </pre>
    </div>
  )
}
