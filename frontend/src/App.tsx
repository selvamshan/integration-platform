import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { Login } from './pages/Login'
import { Dashboard } from './pages/Dashboard'
import { Flows } from './pages/Flows'
import { FlowEditor } from './pages/FlowEditor'
import { Connectors } from './pages/Connectors'
import { AuditLogs } from './pages/AuditLogs'
import { Setup } from './pages/Setup'
import { Users } from './pages/Users'
import { Clients } from './pages/Clients'
import { Layout } from './components/Layout/Layout'
import { ProtectedRoute } from './components/Auth/ProtectedRoute'
import { useSetupStore } from './store/setupStore'

function SetupGuard({ children }: { children: React.ReactNode }) {
  const isConfigured = useSetupStore((s) => s.isConfigured)
  if (!isConfigured) {
    return <Navigate to="/setup" replace />
  }
  return <>{children}</>
}

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/setup" element={<Setup />} />
        <Route
          path="/login"
          element={
            <SetupGuard>
              <Login />
            </SetupGuard>
          }
        />
        <Route
          path="/"
          element={
            <SetupGuard>
              <ProtectedRoute>
                <Layout />
              </ProtectedRoute>
            </SetupGuard>
          }
        >
          <Route index element={<Navigate to="/dashboard" replace />} />
          <Route path="dashboard" element={<Dashboard />} />
          <Route path="flows" element={<Flows />} />
          <Route path="flows/:id" element={<FlowEditor />} />
          <Route path="connectors" element={<Connectors />} />
          <Route path="audit-logs" element={<AuditLogs />} />
          <Route path="users" element={<Users />} />
          <Route path="clients" element={<Clients />} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}

export default App
