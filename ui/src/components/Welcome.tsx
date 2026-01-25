export function Welcome() {
  return (
    <div className="flex items-center justify-center min-h-[60vh]">
      <div className="max-w-lg rounded-lg border border-border bg-card p-6">
        <h2 className="text-2xl font-semibold text-foreground">Welcome to Codeloops</h2>
        <p className="mt-2 text-muted-foreground">
          Run your first actor-critic loop to see sessions appear here in real-time.
        </p>
        <div className="mt-4 bg-muted rounded-md p-4 font-mono text-sm space-y-1">
          <div className="text-muted-foreground">$ cd your-project</div>
          <div className="text-muted-foreground">$ echo "Fix the auth bug" &gt; prompt.md</div>
          <div className="text-foreground">$ codeloops</div>
        </div>
        <p className="mt-4 text-sm text-muted-foreground text-center">
          Once a session starts, it will stream live here.
        </p>
      </div>
    </div>
  )
}
