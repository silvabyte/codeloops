import { Link, useLocation } from 'react-router-dom'
import { cn } from '@/lib/utils'

interface SubNavItem {
  label: string
  path: string
}

interface SubNavProps {
  items: SubNavItem[]
}

/**
 * Horizontal sub-navigation tabs component.
 * Used to provide secondary navigation within a section (e.g., Run Insights).
 */
export function SubNav({ items }: SubNavProps) {
  const location = useLocation()

  return (
    <div className="border-b border-border mb-6">
      <nav className="flex gap-6">
        {items.map((item) => (
          <Link
            key={item.path}
            to={item.path}
            className={cn(
              'pb-3 text-sm transition-colors border-b-2 -mb-px',
              location.pathname === item.path
                ? 'border-primary text-foreground font-medium'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            )}
          >
            {item.label}
          </Link>
        ))}
      </nav>
    </div>
  )
}
