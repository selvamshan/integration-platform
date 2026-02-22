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
