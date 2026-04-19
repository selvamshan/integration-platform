import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export type OidcProvider = 'keycloak' | 'auth0' | 'okta'

export interface OidcConfig {
  provider: OidcProvider
  controlPlaneUrl: string

  // Keycloak
  keycloakUrl: string
  realm: string
  clientId: string
  clientSecret: string

  // Auth0
  auth0Domain: string
  auth0Audience: string

  // Okta
  oktaDomain: string
  oktaAuthServerId: string
  oktaAudience: string
}

interface SetupState extends OidcConfig {
  isConfigured: boolean
  setConfig: (config: Partial<OidcConfig>) => void
  setProvider: (provider: OidcProvider) => void
  markConfigured: () => void
  reset: () => void
}

const DEFAULTS: OidcConfig = {
  provider: 'keycloak',
  controlPlaneUrl: import.meta.env.VITE_CONTROL_PLANE_URL || 'http://localhost:8081',
  // Keycloak
  keycloakUrl: import.meta.env.VITE_KEYCLOAK_URL || 'http://localhost:8180',
  realm: import.meta.env.VITE_KEYCLOAK_REALM || 'integration-platform',
  clientId: import.meta.env.VITE_KEYCLOAK_CLIENT_ID || 'control-plane',
  clientSecret: import.meta.env.VITE_KEYCLOAK_CLIENT_SECRET || '',
  // Auth0
  auth0Domain: '',
  auth0Audience: '',
  // Okta
  oktaDomain: '',
  oktaAuthServerId: 'default',
  oktaAudience: 'api://default',
}

export const useSetupStore = create<SetupState>()(
  persist(
    (set) => ({
      ...DEFAULTS,
      isConfigured: false,
      setConfig: (config) => set((s) => ({ ...s, ...config })),
      setProvider: (provider) => set({ provider }),
      markConfigured: () => set({ isConfigured: true }),
      reset: () => set({ ...DEFAULTS, isConfigured: false }),
    }),
    {
      name: 'oidc-setup',
      version: 1,
      migrate: (persisted: unknown, version: number) => {
        const state = persisted as Partial<OidcConfig & { isConfigured: boolean }>
        if (version < 1) {
          state.controlPlaneUrl = DEFAULTS.controlPlaneUrl
        }
        return state
      },
    }
  )
)

export function getOidcConfig(): OidcConfig {
  const s = useSetupStore.getState()
  return {
    provider: s.provider,
    controlPlaneUrl: s.controlPlaneUrl,
    keycloakUrl: s.keycloakUrl,
    realm: s.realm,
    clientId: s.clientId,
    clientSecret: s.clientSecret,
    auth0Domain: s.auth0Domain,
    auth0Audience: s.auth0Audience,
    oktaDomain: s.oktaDomain,
    oktaAuthServerId: s.oktaAuthServerId,
    oktaAudience: s.oktaAudience,
  }
}
