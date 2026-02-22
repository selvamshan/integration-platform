import axios from 'axios'
import { authService } from './auth'

export const api = axios.create({
  baseURL: import.meta.env.VITE_CONTROL_PLANE_URL || 'http://localhost:8081',
})

// ── helpers ──────────────────────────────────────────────────────────────────

function getStored() {
  try {
    const raw = localStorage.getItem('auth-storage')
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

function getToken(): string | null {
  return getStored()?.state?.token ?? null
}

function getRefreshToken(): string | null {
  return getStored()?.state?.refreshToken ?? null
}

function saveTokens(accessToken: string, refreshToken: string) {
  const stored = getStored() ?? { state: {}, version: 0 }
  stored.state.token = accessToken
  stored.state.refreshToken = refreshToken
  localStorage.setItem('auth-storage', JSON.stringify(stored))
}

function clearAuth() {
  const stored = getStored()
  if (!stored) return
  stored.state.token = null
  stored.state.refreshToken = null
  stored.state.user = null
  localStorage.setItem('auth-storage', JSON.stringify(stored))
}

// ── request interceptor ───────────────────────────────────────────────────────

api.interceptors.request.use((config) => {
  const token = getToken()
  if (token) config.headers.Authorization = `Bearer ${token}`
  return config
})

// ── response interceptor (auto-refresh) ──────────────────────────────────────

let isRefreshing = false
let queue: Array<(token: string) => void> = []

function flushQueue(token: string) {
  queue.forEach((cb) => cb(token))
  queue = []
}

function rejectQueue(err: unknown) {
  queue.forEach((_, i, arr) => {
    // resolve the pending promise with a rejection so callers get the error
    arr[i] = () => { throw err }
  })
  queue = []
}

api.interceptors.response.use(
  (response) => response,
  async (error) => {
    const original = error.config

    // Only handle 401s; don't retry refresh requests themselves
    if (error.response?.status !== 401 || original._retry) {
      return Promise.reject(error)
    }

    const refreshToken = getRefreshToken()
    if (!refreshToken) {
      clearAuth()
      window.location.href = '/login'
      return Promise.reject(error)
    }

    if (isRefreshing) {
      // Queue this request until the ongoing refresh completes
      return new Promise((resolve) => {
        queue.push((token) => {
          original.headers.Authorization = `Bearer ${token}`
          resolve(api(original))
        })
      })
    }

    original._retry = true
    isRefreshing = true

    try {
      const data = await authService.refreshToken(refreshToken)
      saveTokens(data.access_token, data.refresh_token)

      api.defaults.headers.common.Authorization = `Bearer ${data.access_token}`
      original.headers.Authorization = `Bearer ${data.access_token}`

      flushQueue(data.access_token)
      return api(original)
    } catch (refreshError) {
      rejectQueue(refreshError)
      clearAuth()
      window.location.href = '/login'
      return Promise.reject(refreshError)
    } finally {
      isRefreshing = false
    }
  }
)
