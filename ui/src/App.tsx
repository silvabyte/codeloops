import { Routes, Route } from 'react-router-dom'
import { Dashboard } from '@/pages/Dashboard'
import { SessionDetail } from '@/pages/SessionDetail'
import { Stats } from '@/pages/Stats'

function App() {
  return (
    <Routes>
      <Route path="/" element={<Dashboard />} />
      <Route path="/sessions/:id" element={<SessionDetail />} />
      <Route path="/stats" element={<Stats />} />
    </Routes>
  )
}

export default App
