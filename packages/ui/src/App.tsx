import { Routes, Route } from 'react-router-dom'
import { Layout } from '@/components/Layout'
import { ProjectProvider, ProjectRedirect } from '@/hooks/useProject'
import { RunInsights } from '@/pages/RunInsights'
import { SessionDetail } from '@/pages/SessionDetail'
import { Stats } from '@/pages/Stats'
import { PromptBuilder } from '@/pages/PromptBuilder'
import { NotFound } from '@/pages/NotFound'

function App() {
  return (
    <Routes>
      {/* Redirect root to default project */}
      <Route path="/" element={<ProjectRedirect />} />

      {/* Project-scoped routes */}
      <Route path="/projects/:projectId" element={<ProjectProvider><Layout /></ProjectProvider>}>
        <Route index element={<PromptBuilder />} />
        <Route path="run-insights" element={<RunInsights />} />
        <Route path="run-insights/status" element={<Stats />} />
        <Route path="sessions/:id" element={<SessionDetail />} />
      </Route>

      <Route path="*" element={<NotFound />} />
    </Routes>
  )
}

export default App
