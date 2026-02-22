import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { Login } from './pages/Login'
import { Dashboard } from './pages/Dashboard'
import { Flows } from './pages/Flows'
import { FlowEditor } from './pages/FlowEditor'
import { Connectors } from './pages/Connectors'
import { Layout } from './components/Layout/Layout'
import { ProtectedRoute } from './components/Auth/ProtectedRoute'

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route path="/" element={<ProtectedRoute><Layout /></ProtectedRoute>}>
          <Route index element={<Navigate to="/dashboard" replace />} />
          <Route path="dashboard" element={<Dashboard />} />
          <Route path="flows" element={<Flows />} />
          <Route path="flows/:id" element={<FlowEditor />} />
          <Route path="connectors" element={<Connectors />} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}

export default App
