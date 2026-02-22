#!/bin/bash
# Create all missing frontend files

set -e

echo "Creating missing frontend files..."

# Create App.tsx
cat > src/App.tsx << 'EOFAPP'
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom'
import { Login } from './pages/Login'
import { Dashboard } from './pages/Dashboard'
import { Flows } from './pages/Flows'
import { FlowEditor } from './pages/FlowEditor'
import { Connectors } from './pages/Connectors'
import { Layout } from './components/Layout/Layout'
import { ProtectedRoute } from './components/Auth/ProtectedRoute'

function App() {
  return (
    <Router>
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route
          path="/"
          element={
            <ProtectedRoute>
              <Layout />
            </ProtectedRoute>
          }
        >
          <Route index element={<Navigate to="/dashboard" replace />} />
          <Route path="dashboard" element={<Dashboard />} />
          <Route path="flows" element={<Flows />} />
          <Route path="flows/:id" element={<FlowEditor />} />
          <Route path="connectors" element={<Connectors />} />
        </Route>
      </Routes>
    </Router>
  )
}

export default App
EOFAPP

# Create types directory
mkdir -p src/types

cat > src/types/flow.ts << 'EOFFLOW'
export interface Flow {
  id: string
  name: string
  trigger: Trigger
  steps: FlowStep[]
  active?: boolean
  created_at?: string
}

export interface Trigger {
  type: 'http' | 'cron' | 'event'
  path?: string
  method?: string
  schedule?: string
}

export interface FlowStep {
  type: 'log' | 'call' | 'transform'
  name: string
  message?: string
  connector?: string
  operation?: string
  params?: any
  spec?: any
}

export interface FlowDefinition {
  id: string
  name: string
  trigger: Trigger
  steps: FlowStep[]
}
EOFFLOW

cat > src/types/connector.ts << 'EOFCONN'
export interface ConnectorInstance {
  id: string
  name: string
  connector_type: 'postgres' | 'mysql' | 'http'
  host?: string
  port?: number
  database_name?: string
  username?: string
  extra_attributes?: Record<string, any>
  active: boolean
  created_at: string
}

export interface Connector {
  id: string
  name: string
  connector_type: string
  host?: string
  port?: number
  database?: string
  username?: string
  password?: string
  extra_attributes?: Record<string, any>
}
EOFCONN

cat > src/types/auth.ts << 'EOFAUTH'
export interface User {
  sub: string
  email?: string
  name?: string
  preferred_username?: string
  roles?: string[]
}

export interface AuthTokens {
  access_token: string
  refresh_token: string
  expires_in: number
  token_type: string
}
EOFAUTH

# Create services directory
mkdir -p src/services

cat > src/services/api.ts << 'EOFAPI'
import axios from 'axios'
import { useAuthStore } from '../store/authStore'

const API_BASE = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8080'
const CONTROL_PLANE = import.meta.env.VITE_CONTROL_PLANE_URL || 'http://localhost:8081'

export const api = axios.create({
  baseURL: CONTROL_PLANE,
})

export const dataPlaneApi = axios.create({
  baseURL: API_BASE,
})

// Add auth token to requests
api.interceptors.request.use((config) => {
  const token = useAuthStore.getState().token
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

dataPlaneApi.interceptors.request.use((config) => {
  const token = useAuthStore.getState().token
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

// Auto refresh token on 401
api.interceptors.response.use(
  (response) => response,
  async (error) => {
    if (error.response?.status === 401) {
      try {
        await useAuthStore.getState().refresh()
        return api(error.config)
      } catch {
        useAuthStore.getState().logout()
        window.location.href = '/login'
      }
    }
    return Promise.reject(error)
  }
)
EOFAPI

cat > src/services/auth.ts << 'EOFAUTHSVC'
import axios from 'axios'

const KEYCLOAK_URL = import.meta.env.VITE_KEYCLOAK_URL || 'http://localhost:8180'
const REALM = import.meta.env.VITE_KEYCLOAK_REALM || 'integration-platform'
const CLIENT_ID = import.meta.env.VITE_KEYCLOAK_CLIENT_ID || 'control-plane'

export const authService = {
  async login(username: string, password: string) {
    const response = await axios.post(
      `${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/token`,
      new URLSearchParams({
        client_id: CLIENT_ID,
        grant_type: 'password',
        username,
        password,
      }),
      {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      }
    )
    return response.data
  },

  async refreshToken(refreshToken: string) {
    const response = await axios.post(
      `${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/token`,
      new URLSearchParams({
        client_id: CLIENT_ID,
        grant_type: 'refresh_token',
        refresh_token: refreshToken,
      }),
      {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      }
    )
    return response.data
  },

  async getUserInfo(token: string) {
    const response = await axios.get(
      `${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/userinfo`,
      {
        headers: { Authorization: `Bearer ${token}` },
      }
    )
    return response.data
  },
}
EOFAUTHSVC

cat > src/services/flows.ts << 'EOFFLOWSVC'
import { api, dataPlaneApi } from './api'
import { Flow, FlowDefinition } from '../types/flow'

export const flowService = {
  async list(): Promise<{ flows: Flow[] }> {
    const response = await api.get('/flows')
    return response.data
  },

  async get(id: string): Promise<Flow> {
    const response = await api.get(`/flows/${id}`)
    return response.data
  },

  async create(flow: FlowDefinition): Promise<Flow> {
    const response = await api.post('/flows', flow)
    return response.data
  },

  async update(id: string, flow: Partial<FlowDefinition>): Promise<Flow> {
    const response = await api.put(`/flows/${id}`, flow)
    return response.data
  },

  async delete(id: string): Promise<void> {
    await api.delete(`/flows/${id}`)
  },

  async execute(path: string, body?: any): Promise<any> {
    const response = await dataPlaneApi.post(`/api/trigger/${path}`, body)
    return response.data
  },
}
EOFFLOWSVC

cat > src/services/connectors.ts << 'EOFCONNSVC'
import { api } from './api'
import { Connector, ConnectorInstance } from '../types/connector'

export const connectorService = {
  async list(): Promise<{ instances: ConnectorInstance[] }> {
    const response = await api.get('/connector-instances')
    return response.data
  },

  async create(connector: Connector): Promise<ConnectorInstance> {
    const response = await api.post('/connector-instances', connector)
    return response.data
  },

  async update(id: string, connector: Partial<Connector>): Promise<ConnectorInstance> {
    const response = await api.put(`/connector-instances/${id}`, connector)
    return response.data
  },

  async delete(id: string): Promise<void> {
    await api.delete(`/connector-instances/${id}`)
  },
}
EOFCONNSVC

# Create store directory
mkdir -p src/store

cat > src/store/authStore.ts << 'EOFSTORE'
import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { authService } from '../services/auth'
import { User } from '../types/auth'

interface AuthState {
  token: string | null
  refreshToken: string | null
  user: User | null
  login: (username: string, password: string) => Promise<void>
  logout: () => void
  refresh: () => Promise<void>
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      token: null,
      refreshToken: null,
      user: null,

      login: async (username, password) => {
        const data = await authService.login(username, password)
        const user = await authService.getUserInfo(data.access_token)
        set({
          token: data.access_token,
          refreshToken: data.refresh_token,
          user,
        })
      },

      logout: () => {
        set({ token: null, refreshToken: null, user: null })
      },

      refresh: async () => {
        const { refreshToken } = get()
        if (!refreshToken) throw new Error('No refresh token')
        const data = await authService.refreshToken(refreshToken)
        set({ token: data.access_token, refreshToken: data.refresh_token })
      },
    }),
    {
      name: 'auth-storage',
    }
  )
)
EOFSTORE

# Create components directory structure
mkdir -p src/components/{Layout,Auth,Common}
mkdir -p src/pages

# Create Layout component
cat > src/components/Layout/Layout.tsx << 'EOFLAYOUT'
import { Outlet, Link, useNavigate } from 'react-router-dom'
import { useAuthStore } from '../../store/authStore'
import { LogOut, LayoutDashboard, GitBranch, Database } from 'lucide-react'

export function Layout() {
  const { user, logout } = useAuthStore()
  const navigate = useNavigate()

  const handleLogout = () => {
    logout()
    navigate('/login')
  }

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Navbar */}
      <nav className="bg-white border-b border-gray-200">
        <div className="mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between h-16">
            <div className="flex">
              <div className="flex-shrink-0 flex items-center">
                <h1 className="text-xl font-bold text-primary-600">Integration Platform</h1>
              </div>
              <div className="hidden sm:ml-6 sm:flex sm:space-x-8">
                <Link
                  to="/dashboard"
                  className="inline-flex items-center px-1 pt-1 border-b-2 border-transparent text-sm font-medium text-gray-500 hover:border-gray-300 hover:text-gray-700"
                >
                  <LayoutDashboard className="w-4 h-4 mr-2" />
                  Dashboard
                </Link>
                <Link
                  to="/flows"
                  className="inline-flex items-center px-1 pt-1 border-b-2 border-transparent text-sm font-medium text-gray-500 hover:border-gray-300 hover:text-gray-700"
                >
                  <GitBranch className="w-4 h-4 mr-2" />
                  Flows
                </Link>
                <Link
                  to="/connectors"
                  className="inline-flex items-center px-1 pt-1 border-b-2 border-transparent text-sm font-medium text-gray-500 hover:border-gray-300 hover:text-gray-700"
                >
                  <Database className="w-4 h-4 mr-2" />
                  Connectors
                </Link>
              </div>
            </div>
            <div className="flex items-center">
              <span className="text-sm text-gray-700 mr-4">{user?.name || user?.preferred_username}</span>
              <button onClick={handleLogout} className="btn btn-secondary flex items-center gap-2">
                <LogOut className="w-4 h-4" />
                Logout
              </button>
            </div>
          </div>
        </div>
      </nav>

      {/* Main content */}
      <main className="py-10">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <Outlet />
        </div>
      </main>
    </div>
  )
}
EOFLAYOUT

# Create ProtectedRoute
cat > src/components/Auth/ProtectedRoute.tsx << 'EOFPROTECT'
import { Navigate } from 'react-router-dom'
import { useAuthStore } from '../../store/authStore'

export function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const token = useAuthStore((state) => state.token)

  if (!token) {
    return <Navigate to="/login" replace />
  }

  return <>{children}</>
}
EOFPROTECT

# Create Login page
cat > src/pages/Login.tsx << 'EOFLOGIN'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { useAuthStore } from '../store/authStore'
import { useNavigate } from 'react-router-dom'
import { useState } from 'react'

const loginSchema = z.object({
  username: z.string().min(1, 'Username required'),
  password: z.string().min(1, 'Password required'),
})

type LoginForm = z.infer<typeof loginSchema>

export function Login() {
  const { register, handleSubmit, formState: { errors } } = useForm<LoginForm>({
    resolver: zodResolver(loginSchema),
  })
  const login = useAuthStore((state) => state.login)
  const navigate = useNavigate()
  const [error, setError] = useState('')

  const onSubmit = async (data: LoginForm) => {
    try {
      setError('')
      await login(data.username, data.password)
      navigate('/dashboard')
    } catch (err: any) {
      setError(err.response?.data?.error_description || 'Login failed')
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-100">
      <div className="card max-w-md w-full">
        <h1 className="text-2xl font-bold mb-6 text-center">Integration Platform</h1>
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4">
            {error}
          </div>
        )}
        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1">Username</label>
            <input {...register('username')} className="input" placeholder="admin@local.dev" />
            {errors.username && (
              <p className="text-red-500 text-sm mt-1">{errors.username.message}</p>
            )}
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">Password</label>
            <input {...register('password')} type="password" className="input" />
            {errors.password && (
              <p className="text-red-500 text-sm mt-1">{errors.password.message}</p>
            )}
          </div>
          <button type="submit" className="btn btn-primary w-full">
            Login
          </button>
        </form>
        <p className="text-sm text-gray-500 mt-4 text-center">
          Default: admin@local.dev / admin123
        </p>
      </div>
    </div>
  )
}
EOFLOGIN

# Create Dashboard page
cat > src/pages/Dashboard.tsx << 'EOFDASH'
import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { flowService } from '../services/flows'
import { connectorService } from '../services/connectors'
import { GitBranch, Database, Activity } from 'lucide-react'

export function Dashboard() {
  const [stats, setStats] = useState({ flows: 0, connectors: 0 })

  useEffect(() => {
    const load = async () => {
      try {
        const [flows, connectors] = await Promise.all([
          flowService.list(),
          connectorService.list(),
        ])
        setStats({
          flows: flows.flows?.length || 0,
          connectors: connectors.instances?.length || 0,
        })
      } catch (error) {
        console.error('Failed to load stats:', error)
      }
    }
    load()
  }, [])

  return (
    <div>
      <h1 className="text-3xl font-bold mb-8">Dashboard</h1>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-500">Total Flows</p>
              <p className="text-3xl font-bold">{stats.flows}</p>
            </div>
            <GitBranch className="w-12 h-12 text-primary-600" />
          </div>
        </div>

        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-500">Connectors</p>
              <p className="text-3xl font-bold">{stats.connectors}</p>
            </div>
            <Database className="w-12 h-12 text-primary-600" />
          </div>
        </div>

        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-500">Status</p>
              <p className="text-3xl font-bold text-green-600">Active</p>
            </div>
            <Activity className="w-12 h-12 text-green-600" />
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="card">
          <h2 className="text-lg font-semibold mb-4">Quick Actions</h2>
          <div className="space-y-2">
            <Link to="/flows" className="block p-3 rounded hover:bg-gray-50 border">
              Create New Flow
            </Link>
            <Link to="/connectors" className="block p-3 rounded hover:bg-gray-50 border">
              Add Connector
            </Link>
          </div>
        </div>

        <div className="card">
          <h2 className="text-lg font-semibold mb-4">Recent Activity</h2>
          <p className="text-gray-500">No recent activity</p>
        </div>
      </div>
    </div>
  )
}
EOFDASH

# Create placeholder pages
cat > src/pages/Flows.tsx << 'EOFFLOWS'
import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { flowService } from '../services/flows'
import { Flow } from '../types/flow'
import { Plus, Play } from 'lucide-react'

export function Flows() {
  const [flows, setFlows] = useState<Flow[]>([])

  useEffect(() => {
    const load = async () => {
      try {
        const data = await flowService.list()
        setFlows(data.flows || [])
      } catch (error) {
        console.error('Failed to load flows:', error)
      }
    }
    load()
  }, [])

  return (
    <div>
      <div className="flex justify-between items-center mb-8">
        <h1 className="text-3xl font-bold">Flows</h1>
        <Link to="/flows/new" className="btn btn-primary flex items-center gap-2">
          <Plus className="w-4 h-4" />
          Create Flow
        </Link>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {flows.map((flow) => (
          <div key={flow.id} className="card hover:shadow-md transition-shadow">
            <div className="flex justify-between items-start mb-4">
              <div>
                <h3 className="text-lg font-semibold">{flow.name}</h3>
                <p className="text-sm text-gray-500">{flow.trigger.type.toUpperCase()}</p>
              </div>
              {flow.active && (
                <span className="px-2 py-1 bg-green-100 text-green-800 text-xs rounded">Active</span>
              )}
            </div>
            <p className="text-sm text-gray-600 mb-4">{flow.steps?.length || 0} steps</p>
            <div className="flex gap-2">
              <Link to={`/flows/${flow.id}`} className="btn btn-secondary text-sm flex-1">
                Edit
              </Link>
              <button className="btn btn-primary text-sm flex items-center gap-2">
                <Play className="w-3 h-3" />
                Test
              </button>
            </div>
          </div>
        ))}
      </div>

      {flows.length === 0 && (
        <div className="text-center py-12">
          <p className="text-gray-500">No flows yet. Create your first flow!</p>
        </div>
      )}
    </div>
  )
}
EOFFLOWS

cat > src/pages/FlowEditor.tsx << 'EOFEDITOR'
export function FlowEditor() {
  return (
    <div>
      <h1 className="text-3xl font-bold mb-8">Flow Editor</h1>
      <p className="text-gray-500">Flow editor with ReactFlow coming soon...</p>
    </div>
  )
}
EOFEDITOR

cat > src/pages/Connectors.tsx << 'EOFCONNPAGE'
import { useEffect, useState } from 'react'
import { connectorService } from '../services/connectors'
import { ConnectorInstance } from '../types/connector'
import { Plus, Database } from 'lucide-react'

export function Connectors() {
  const [connectors, setConnectors] = useState<ConnectorInstance[]>([])

  useEffect(() => {
    const load = async () => {
      try {
        const data = await connectorService.list()
        setConnectors(data.instances || [])
      } catch (error) {
        console.error('Failed to load connectors:', error)
      }
    }
    load()
  }, [])

  return (
    <div>
      <div className="flex justify-between items-center mb-8">
        <h1 className="text-3xl font-bold">Connectors</h1>
        <button className="btn btn-primary flex items-center gap-2">
          <Plus className="w-4 h-4" />
          Add Connector
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {connectors.map((connector) => (
          <div key={connector.id} className="card">
            <div className="flex items-start justify-between mb-4">
              <div className="flex items-center gap-3">
                <Database className="w-8 h-8 text-primary-600" />
                <div>
                  <h3 className="font-semibold">{connector.name}</h3>
                  <p className="text-sm text-gray-500">{connector.connector_type.toUpperCase()}</p>
                </div>
              </div>
              {connector.active && (
                <span className="px-2 py-1 bg-green-100 text-green-800 text-xs rounded">Active</span>
              )}
            </div>
            {connector.host && (
              <p className="text-sm text-gray-600">{connector.host}:{connector.port}</p>
            )}
          </div>
        ))}
      </div>

      {connectors.length === 0 && (
        <div className="text-center py-12">
          <p className="text-gray-500">No connectors yet. Add your first connector!</p>
        </div>
      )}
    </div>
  )
}
EOFCONNPAGE

echo "✅ All files created!"
echo ""
echo "Now run:"
echo "  npm install"
echo "  npm run dev"

