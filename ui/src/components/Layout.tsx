import { Outlet, Link, useLocation } from 'react-router-dom'
import { cn } from '@/lib/utils'

export function Layout() {
  const location = useLocation()

  const navItems = [
    { path: '/', label: 'Prompts' },
    { path: '/run-insights', label: 'Run Insights' },
  ]

  const isActive = (path: string) => {
    if (path === '/') {
      return location.pathname === '/'
    }
    // Match /run-insights and /run-insights/status
    return location.pathname.startsWith(path)
  }

  return (
    <div className="min-h-screen bg-background">
      <header className="border-b border-border">
        <div className="px-6 py-4 flex items-center justify-between">
          <Link to="/" className="hover:opacity-80 transition-opacity">
            <img src="/icon.svg" alt="Codeloops" className="h-7 w-auto" />
          </Link>
          <nav className="flex items-center gap-6">
            {navItems.map((item) => (
              <Link
                key={item.path}
                to={item.path}
                className={cn(
                  'text-sm transition-colors',
                  isActive(item.path)
                    ? 'text-foreground font-medium'
                    : 'text-muted-foreground hover:text-foreground'
                )}
              >
                {item.label}
              </Link>
            ))}
          </nav>
        </div>
      </header>
      <main>
        <Outlet />
      </main>
    </div>
  )
}
