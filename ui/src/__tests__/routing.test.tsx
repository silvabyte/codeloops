import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import App from '../App'

// Mock session data for RunInsights to show SubNav (requires at least one session)
const mockSession = {
  id: 'test-session-1',
  workingDir: '/test/project',
  project: 'test-project',
  outcome: 'success',
  timestamp: '2024-01-01T00:00:00Z',
  iterations: 3,
  promptPreview: 'Test prompt preview',
  durationSecs: 120,
  confidence: 0.95,
  actorAgent: 'Claude Code',
  criticAgent: 'Claude Code',
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
      totalSessions: 10,
      successRate: 0.85,
      avgIterations: 3.5,
      avgDurationSecs: 120,
      sessionsOverTime: [],
      byProject: [],
    },
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}))

vi.mock('@/hooks/useMetrics', () => ({
  useMetrics: () => ({
    metrics: {
      totalSessions: 10,
      successfulSessions: 8,
      successRate: 0.8,
      firstTrySuccessRate: 0.5,
      avgIterationsToSuccess: 1.5,
      avgCycleTimeSecs: 120,
      wasteRate: 0.1,
      totalIterations: 20,
      criticApprovalRate: 0.6,
      avgFeedbackLength: 150,
      improvementRate: 0.75,
      sessionsOverTime: [],
      byProject: [],
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
    },
    isSaving: false,
    error: null,
    isStreaming: false,
    selectWorkType: vi.fn(),
    sendMessage: vi.fn(),
    updatePromptDraft: vi.fn(),
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

    it('should show main navigation with Prompts active', async () => {
      renderApp('/')

      await waitFor(() => {
        expect(screen.getByRole('link', { name: /prompts/i })).toBeInTheDocument()
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
        // Stats page shows the Run Insights header (h1) and DORA-inspired metrics sections
        expect(screen.getByRole('heading', { name: 'Run Insights', level: 1 })).toBeInTheDocument()
        expect(screen.getByText('Session Efficacy')).toBeInTheDocument()
        expect(screen.getByText('Critic Performance')).toBeInTheDocument()
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
        expect(screen.getByRole('link', { name: /prompts/i })).toBeInTheDocument()
        expect(screen.getByRole('link', { name: /run insights/i })).toBeInTheDocument()
      })
    })

    it('should have correct navigation paths', async () => {
      renderApp('/')

      await waitFor(() => {
        const promptsLink = screen.getByRole('link', { name: /prompts/i })
        const runInsightsLink = screen.getByRole('link', { name: /run insights/i })

        expect(promptsLink).toHaveAttribute('href', '/')
        expect(runInsightsLink).toHaveAttribute('href', '/run-insights')
      })
    })
  })
})
