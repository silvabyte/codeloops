import { Routes, Route } from 'react-router-dom'
import { Layout } from '@/components/Layout'
import { Dashboard } from '@/pages/Dashboard'
import { SessionDetail } from '@/pages/SessionDetail'
import { Stats } from '@/pages/Stats'
import { PromptBuilder } from '@/pages/PromptBuilder'
import { NotFound } from '@/pages/NotFound'

function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route path="/" element={<Dashboard />} />
        <Route path="/sessions/:id" element={<SessionDetail />} />
        <Route path="/stats" element={<Stats />} />
        <Route path="/prompt-builder" element={<PromptBuilder />} />
        <Route path="*" element={<NotFound />} />
      </Route>
    </Routes>
  )
}

export default App
