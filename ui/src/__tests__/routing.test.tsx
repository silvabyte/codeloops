import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import App from '../App'

// Mock session data for RunInsights to show SubNav (requires at least one session)
const mockSession = {
  id: 'test-session-1',
  working_dir: '/test/project',
  project_name: 'test-project',
  status: 'completed',
  started_at: '2024-01-01T00:00:00Z',
  iterations: 3,
}

// Mock the hooks that make API calls to avoid network requests
vi.mock('@/hooks/useSessions', () => ({
  useSessions: () => ({
    sessions: [mockSession],
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}))

vi.mock('@/hooks/useStats', () => ({
  useStats: () => ({
    stats: {
      total_sessions: 10,
      success_rate: 0.85,
      avg_iterations: 3.5,
      avg_duration_secs: 120,
      sessions_over_time: [],
      sessions_by_status: [],
      sessions_by_exit_reason: [],
      by_project: [],
    },
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}))

vi.mock('@/hooks/useSSE', () => ({
  useSessionEvents: () => {},
}))

vi.mock('@/hooks/usePromptSession', () => ({
  usePromptSession: () => ({
    // Return state with proper structure for the state machine
    state: {
      status: 'selecting_work_type',
      workingDir: '/test/project',
      projectName: 'test-project',
    },
    session: {
      id: null,
      workType: null,
      workingDir: '/test/project',
      projectName: 'test-project',
      messages: [],
      promptDraft: '',
      status: 'selecting',
      previewOpen: false,
    },
    isSaving: false,
    error: null,
    isStreaming: false,
    selectWorkType: vi.fn(),
    sendMessage: vi.fn(),
    updatePromptDraft: vi.fn(),
    togglePreview: vi.fn(),
    closePreview: vi.fn(),
    save: vi.fn(),
    clearError: vi.fn(),
    newPrompt: vi.fn(),
    loadPrompt: vi.fn(),
  }),
}))

function renderApp(initialRoute: string) {
  return render(
    <MemoryRouter initialEntries={[initialRoute]}>
      <App />
    </MemoryRouter>
  )
}

describe('App Routing', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('default route (/)', () => {
    it('should render PromptBuilder at root path', async () => {
      renderApp('/')

      // PromptBuilder shows work type selector with "What are you building?" heading
      await waitFor(() => {
        expect(screen.getByText('What are you building?')).toBeInTheDocument()
      })
    })

    it('should show main navigation with New Prompt active', async () => {
      renderApp('/')

      await waitFor(() => {
        expect(screen.getByRole('link', { name: /new prompt/i })).toBeInTheDocument()
      })
    })
  })

  describe('/run-insights route', () => {
    it('should render RunInsights page', async () => {
      renderApp('/run-insights')

      await waitFor(() => {
        // RunInsights shows Sub-navigation with Overview/Status tabs
        expect(screen.getByRole('link', { name: 'Overview' })).toBeInTheDocument()
        expect(screen.getByRole('link', { name: 'Status' })).toBeInTheDocument()
      })
    })

    it('should highlight Run Insights in main nav', async () => {
      renderApp('/run-insights')

      await waitFor(() => {
        const runInsightsLink = screen.getByRole('link', { name: /run insights/i })
        expect(runInsightsLink).toBeInTheDocument()
      })
    })

    it('should show Overview tab as active', async () => {
      renderApp('/run-insights')

      await waitFor(() => {
        const overviewTab = screen.getByRole('link', { name: 'Overview' })
        // SectionHeader uses amber background for active tabs
        expect(overviewTab).toHaveClass('bg-amber/10')
      })
    })
  })

  describe('/run-insights/status route', () => {
    it('should render Stats page', async () => {
      renderApp('/run-insights/status')

      await waitFor(() => {
        // Stats page shows the Run Insights header (h1) and summary cards
        expect(screen.getByRole('heading', { name: 'Run Insights', level: 1 })).toBeInTheDocument()
        expect(screen.getByText('Total Sessions')).toBeInTheDocument()
        expect(screen.getByText('Success Rate')).toBeInTheDocument()
      })
    })

    it('should show Status tab as active', async () => {
      renderApp('/run-insights/status')

      await waitFor(() => {
        const statusTab = screen.getByRole('link', { name: 'Status' })
        // SectionHeader uses amber background for active tabs
        expect(statusTab).toHaveClass('bg-amber/10')
      })
    })

    it('should show sub-navigation tabs', async () => {
      renderApp('/run-insights/status')

      await waitFor(() => {
        expect(screen.getByRole('link', { name: 'Overview' })).toBeInTheDocument()
        expect(screen.getByRole('link', { name: 'Status' })).toBeInTheDocument()
      })
    })
  })

  describe('unknown routes', () => {
    it('should render NotFound page for unknown paths', async () => {
      renderApp('/some/unknown/path')

      await waitFor(() => {
        expect(screen.getByText('404')).toBeInTheDocument()
        expect(screen.getByText('Page not found')).toBeInTheDocument()
      })
    })

    it('should have link back to home', async () => {
      renderApp('/invalid-route')

      await waitFor(() => {
        const homeLink = screen.getByRole('link', { name: /back to home/i })
        expect(homeLink).toHaveAttribute('href', '/')
      })
    })
  })

  describe('navigation structure', () => {
    it('should show navigation items', async () => {
      renderApp('/')

      await waitFor(() => {
        expect(screen.getByRole('link', { name: /new prompt/i })).toBeInTheDocument()
        expect(screen.getByRole('link', { name: /run insights/i })).toBeInTheDocument()
      })
    })

    it('should have correct navigation paths', async () => {
      renderApp('/')

      await waitFor(() => {
        const newPromptLink = screen.getByRole('link', { name: /new prompt/i })
        const runInsightsLink = screen.getByRole('link', { name: /run insights/i })

        expect(newPromptLink).toHaveAttribute('href', '/')
        expect(runInsightsLink).toHaveAttribute('href', '/run-insights')
      })
    })
  })
})
