import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { authService } from '@/services/auth'
import { getOidcConfig } from '@/store/setupStore'
import { User } from '@/types/auth'

/** Decode a JWT payload without verifying the signature (UI display only). */
function decodeJwtPayload(token: string): Record<string, unknown> {
  try {
    const base64 = token.split('.')[1].replace(/-/g, '+').replace(/_/g, '/')
    return JSON.parse(atob(base64))
  } catch {
    return {}
  }
}

/** Extract roles from the JWT payload for the active OIDC provider. */
function extractRolesFromToken(token: string): string[] {
  const payload = decodeJwtPayload(token)
  const provider = getOidcConfig().provider
  const clientId = getOidcConfig().clientId
  const roles: string[] = []

  if (provider === 'keycloak') {
    // realm_access.roles
    const realmRoles = (payload.realm_access as any)?.roles
    if (Array.isArray(realmRoles)) roles.push(...realmRoles)
    // resource_access[clientId].roles
    const clientRoles = (payload.resource_access as any)?.[clientId]?.roles
    if (Array.isArray(clientRoles)) roles.push(...clientRoles)
  } else if (provider === 'auth0') {
    const ns = 'https://integration-platform/roles'
    const nsRoles = (payload as any)[ns]
    if (Array.isArray(nsRoles)) roles.push(...nsRoles)
    if (Array.isArray(payload.roles)) roles.push(...(payload.roles as string[]))
  } else if (provider === 'okta') {
    if (Array.isArray(payload.groups)) roles.push(...(payload.groups as string[]))
    if (Array.isArray(payload.roles)) roles.push(...(payload.roles as string[]))
  }

  return [...new Set(roles)]
}

interface AuthState {
  token: string | null
  refreshToken: string | null
  user: User | null
  isLoading: boolean
  error: string | null
  
  login: (username: string, password: string) => Promise<void>
  logout: () => Promise<void>
  clearError: () => void
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      token: null,
      refreshToken: null,
      user: null,
      isLoading: false,
      error: null,

      login: async (username: string, password: string) => {
        try {
          set({ isLoading: true, error: null })
          
          // Get tokens from Keycloak
          const tokenData = await authService.login(username, password)
          
          const userInfo = await authService.getUserInfo(tokenData.access_token)
          const roles = extractRolesFromToken(tokenData.access_token)

          set({
            token: tokenData.access_token,
            refreshToken: tokenData.refresh_token,
            user: { ...userInfo, roles },
            isLoading: false,
          })
        } catch (error: any) {
          const errorMessage = error.response?.data?.error_description || 
                              error.response?.data?.message ||
                              'Login failed'
          set({ 
            error: errorMessage,
            isLoading: false,
            token: null,
            refreshToken: null,
            user: null,
          })
          throw error
        }
      },

      logout: async () => {
        const { token, refreshToken } = get()
        
        try {
          if (token && refreshToken) {
            await authService.logout(token, refreshToken)
          }
        } catch (err) {
          console.error('Logout error:', err)
        } finally {
          set({
            token: null,
            refreshToken: null,
            user: null,
            error: null,
          })
        }
      },

      clearError: () => set({ error: null }),
    }),
    {
      name: 'auth-storage',
      partialize: (state) => ({
        token: state.token,
        refreshToken: state.refreshToken,
        user: state.user,
      }),
    }
  )
)
