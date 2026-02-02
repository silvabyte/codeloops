import { Routes, Route } from 'react-router-dom'
import { Layout } from '@/components/Layout'
import { RunInsights } from '@/pages/RunInsights'
import { SessionDetail } from '@/pages/SessionDetail'
import { Stats } from '@/pages/Stats'
import { PromptBuilder } from '@/pages/PromptBuilder'
import { NotFound } from '@/pages/NotFound'

function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route path="/" element={<PromptBuilder />} />
        <Route path="/run-insights" element={<RunInsights />} />
        <Route path="/run-insights/status" element={<Stats />} />
        <Route path="/sessions/:id" element={<SessionDetail />} />
        <Route path="*" element={<NotFound />} />
      </Route>
    </Routes>
  )
}

export default App
