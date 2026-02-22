import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { authService } from '@/services/auth'
import { User } from '@/types/auth'

interface AuthState {
  token: string | null
  refreshToken: string | null
  user: User | null
  isLoading: boolean
  error: string | null
  
  login: (username: string, password: string) => Promise<void>
  logout: () => Promise<void>
  refresh: () => Promise<void>
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
          
          // Get user info from Keycloak
          const userInfo = await authService.getUserInfo(tokenData.access_token)
          
          // Optionally validate with Control Plane
          try {
            await authService.validateWithControlPlane(tokenData.access_token)
          } catch (err) {
            console.warn('Control Plane validation failed:', err)
          }
          
          set({
            token: tokenData.access_token,
            refreshToken: tokenData.refresh_token,
            user: userInfo,
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

      refresh: async () => {
        const { refreshToken } = get()
        
        if (!refreshToken) {
          console.error('❌ No refresh token available in store')
          throw new Error('No refresh token available')
        }
        
        try {
          console.log('🔄 Calling Keycloak refresh token endpoint...')
          const tokenData = await authService.refreshToken(refreshToken)
          
          if (!tokenData.access_token) {
            throw new Error('No access token in refresh response')
          }
          
          console.log('✅ Got new tokens from Keycloak')
          
          set({
            token: tokenData.access_token,
            refreshToken: tokenData.refresh_token || refreshToken, // Keep old if not provided
          })
        } catch (error: any) {
          // Refresh failed, logout user
          console.error('❌ Token refresh failed:', error.response?.data || error.message)
          
          set({
            token: null,
            refreshToken: null,
            user: null,
            error: 'Session expired',
          })
          throw error
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

// Auto-refresh token before expiry (optional enhancement)
// Check every 5 minutes if token needs refresh
if (typeof window !== 'undefined') {
  setInterval(() => {
    const { token, refresh } = useAuthStore.getState()
    if (token) {
      // Decode JWT to check expiry (simplified)
      try {
        const payload = JSON.parse(atob(token.split('.')[1]))
        const expiryTime = payload.exp * 1000
        const now = Date.now()
        
        // Refresh if token expires in less than 5 minutes
        if (expiryTime - now < 5 * 60 * 1000) {
          refresh().catch(console.error)
        }
      } catch (err) {
        console.error('Token decode error:', err)
      }
    }
  }, 5 * 60 * 1000) // Check every 5 minutes
}
