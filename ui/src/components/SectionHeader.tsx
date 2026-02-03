import type { ReactNode } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { cn } from '@/lib/utils'

interface TabItem {
  label: string
  path: string
}

interface ActionItem {
  label: string
  onClick?: () => void
  disabled?: boolean
  active?: boolean
  hint?: string
}

interface BreadcrumbSegment {
  label: string
  path?: string
  icon?: ReactNode
}

interface SectionHeaderProps {
  /**
   * Left side content - can be:
   * - A string (rendered as title)
   * - Breadcrumb segments array (rendered as styled breadcrumb)
   * - Custom ReactNode for full control
   */
  context: string | BreadcrumbSegment[] | ReactNode

  /**
   * Right side navigation tabs - mutually exclusive with actions
   */
  tabs?: TabItem[]

  /**
   * Right side action buttons - mutually exclusive with tabs
   */
  actions?: ActionItem[]

  /**
   * Additional className for the container
   */
  className?: string
}

function isString(value: unknown): value is string {
  return typeof value === 'string'
}

function isBreadcrumbArray(value: unknown): value is BreadcrumbSegment[] {
  return (
    Array.isArray(value) &&
    value.length > 0 &&
    typeof value[0] === 'object' &&
    'label' in value[0]
  )
}

/**
 * Unified section header component for consistent navigation across pages.
 *
 * Layout:
 * ┌─────────────────────────────────────────────────────────────┐
 * │  [Context/Breadcrumb]                        [Actions/Tabs] │
 * └─────────────────────────────────────────────────────────────┘
 *
 * Follows Amber Circuit design system with amber primary accent.
 */
export function SectionHeader({
  context,
  tabs,
  actions,
  className,
}: SectionHeaderProps) {
  const location = useLocation()

  // Render context (left side)
  const renderContext = () => {
    if (isString(context)) {
      return (
        <h1 className="text-lg font-semibold text-foreground tracking-tight truncate">
          {context}
        </h1>
      )
    }

    if (isBreadcrumbArray(context)) {
      return (
        <nav className="flex items-center gap-1.5 text-sm">
          {context.map((segment, index) => (
            <span key={index} className="flex items-center gap-1.5">
              {index > 0 && (
                <span className="text-dim select-none">/</span>
              )}
              {segment.icon && (
                <span className="text-amber opacity-80">{segment.icon}</span>
              )}
              {segment.path ? (
                <Link
                  to={segment.path}
                  className="text-muted-foreground hover:text-foreground transition-colors"
                >
                  {segment.label}
                </Link>
              ) : (
                <span className="text-foreground font-medium">
                  {segment.label}
                </span>
              )}
            </span>
          ))}
        </nav>
      )
    }

    // Custom ReactNode
    return context
  }

  // Render tabs (right side option 1)
  const renderTabs = () => {
    if (!tabs || tabs.length === 0) return null

    return (
      <nav className="flex items-center gap-1">
        {tabs.map((tab) => {
          const isActive = location.pathname === tab.path
          return (
            <Link
              key={tab.path}
              to={tab.path}
              className={cn(
                'px-3 py-1.5 text-sm rounded-md transition-all duration-150',
                isActive
                  ? 'bg-amber/10 text-amber font-medium'
                  : 'text-muted-foreground hover:text-foreground hover:bg-hover/50'
              )}
            >
              {tab.label}
            </Link>
          )
        })}
      </nav>
    )
  }

  // Render actions (right side option 2)
  const renderActions = () => {
    if (!actions || actions.length === 0) return null

    return (
      <div className="flex items-center gap-1">
        {actions.map((action, index) => (
          <button
            key={index}
            onClick={action.onClick}
            disabled={action.disabled}
            className={cn(
              'px-3 py-1.5 text-sm rounded-md transition-all duration-150',
              action.disabled
                ? 'text-dim cursor-not-allowed'
                : action.active
                  ? 'text-amber font-medium'
                  : 'text-muted-foreground hover:text-foreground hover:bg-hover/50'
            )}
          >
            {action.label}
            {action.hint && (
              <span className="ml-1.5 text-xs text-dim">
                {action.hint}
              </span>
            )}
          </button>
        ))}
      </div>
    )
  }

  return (
    <header
      className={cn(
        'flex items-center justify-between px-6 py-3',
        'border-b border-border bg-background/80 backdrop-blur-sm',
        className
      )}
    >
      <div className="flex items-center min-w-0">{renderContext()}</div>
      <div className="flex items-center gap-2">
        {renderTabs()}
        {renderActions()}
      </div>
    </header>
  )
}
