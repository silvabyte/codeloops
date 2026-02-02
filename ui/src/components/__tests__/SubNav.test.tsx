import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { SubNav } from '../SubNav'

const testItems = [
  { label: 'Overview', path: '/run-insights' },
  { label: 'Status', path: '/run-insights/status' },
]

function renderSubNav(currentPath: string) {
  return render(
    <MemoryRouter initialEntries={[currentPath]}>
      <SubNav items={testItems} />
    </MemoryRouter>
  )
}

describe('SubNav', () => {
  describe('rendering', () => {
    it('should render all navigation tabs', () => {
      renderSubNav('/run-insights')

      expect(screen.getByText('Overview')).toBeInTheDocument()
      expect(screen.getByText('Status')).toBeInTheDocument()
    })

    it('should render tabs as links', () => {
      renderSubNav('/run-insights')

      const overviewLink = screen.getByRole('link', { name: 'Overview' })
      const statusLink = screen.getByRole('link', { name: 'Status' })

      expect(overviewLink).toHaveAttribute('href', '/run-insights')
      expect(statusLink).toHaveAttribute('href', '/run-insights/status')
    })
  })

  describe('active tab highlighting', () => {
    it('should highlight Overview tab when on /run-insights', () => {
      renderSubNav('/run-insights')

      const overviewLink = screen.getByRole('link', { name: 'Overview' })
      const statusLink = screen.getByRole('link', { name: 'Status' })

      // Active tab should have primary border and foreground text
      expect(overviewLink).toHaveClass('border-primary')
      expect(overviewLink).toHaveClass('text-foreground')
      expect(overviewLink).toHaveClass('font-medium')

      // Inactive tab should have transparent border and muted text
      expect(statusLink).toHaveClass('border-transparent')
      expect(statusLink).toHaveClass('text-muted-foreground')
    })

    it('should highlight Status tab when on /run-insights/status', () => {
      renderSubNav('/run-insights/status')

      const overviewLink = screen.getByRole('link', { name: 'Overview' })
      const statusLink = screen.getByRole('link', { name: 'Status' })

      // Status tab should be active
      expect(statusLink).toHaveClass('border-primary')
      expect(statusLink).toHaveClass('text-foreground')
      expect(statusLink).toHaveClass('font-medium')

      // Overview tab should be inactive
      expect(overviewLink).toHaveClass('border-transparent')
      expect(overviewLink).toHaveClass('text-muted-foreground')
    })

    it('should not highlight any tab when on unrelated path', () => {
      renderSubNav('/some-other-path')

      const overviewLink = screen.getByRole('link', { name: 'Overview' })
      const statusLink = screen.getByRole('link', { name: 'Status' })

      // Both tabs should be inactive
      expect(overviewLink).toHaveClass('border-transparent')
      expect(statusLink).toHaveClass('border-transparent')
    })
  })

  describe('navigation structure', () => {
    it('should render inside a nav element', () => {
      renderSubNav('/run-insights')

      const nav = screen.getByRole('navigation')
      expect(nav).toBeInTheDocument()
      expect(nav).toContainElement(screen.getByText('Overview'))
      expect(nav).toContainElement(screen.getByText('Status'))
    })

    it('should render with correct number of items', () => {
      renderSubNav('/run-insights')

      const links = screen.getAllByRole('link')
      expect(links).toHaveLength(2)
    })
  })

  describe('with different items', () => {
    it('should render custom navigation items', () => {
      const customItems = [
        { label: 'Tab A', path: '/section/a' },
        { label: 'Tab B', path: '/section/b' },
        { label: 'Tab C', path: '/section/c' },
      ]

      render(
        <MemoryRouter initialEntries={['/section/b']}>
          <SubNav items={customItems} />
        </MemoryRouter>
      )

      expect(screen.getByText('Tab A')).toBeInTheDocument()
      expect(screen.getByText('Tab B')).toBeInTheDocument()
      expect(screen.getByText('Tab C')).toBeInTheDocument()

      // Tab B should be active
      const tabB = screen.getByRole('link', { name: 'Tab B' })
      expect(tabB).toHaveClass('border-primary')
    })
  })
})
