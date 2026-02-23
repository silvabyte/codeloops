interface MetricCardProps {
  value: string
  label: string
}

export function MetricCard({ value, label }: MetricCardProps) {
  return (
    <div className="text-center">
      <div className="font-mono text-2xl tabular-nums text-foreground">{value}</div>
      <div className="text-xs text-muted-foreground uppercase tracking-wider mt-1">{label}</div>
    </div>
  )
}
