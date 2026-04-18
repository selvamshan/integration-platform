import axios from 'axios'
import { getOidcConfig, OidcConfig } from '@/store/setupStore'

function tokenUrl(cfg: OidcConfig): string {
  switch (cfg.provider) {
    case 'auth0':
      return `https://${cfg.auth0Domain}/oauth/token`
    case 'okta':
      return `https://${cfg.oktaDomain}/oauth2/${cfg.oktaAuthServerId}/v1/token`
    default:
      return `${cfg.keycloakUrl}/realms/${cfg.realm}/protocol/openid-connect/token`
  }
}

function userInfoUrl(cfg: OidcConfig): string {
  switch (cfg.provider) {
    case 'auth0':
      return `https://${cfg.auth0Domain}/userinfo`
    case 'okta':
      return `https://${cfg.oktaDomain}/oauth2/${cfg.oktaAuthServerId}/v1/userinfo`
    default:
      return `${cfg.keycloakUrl}/realms/${cfg.realm}/protocol/openid-connect/userinfo`
  }
}

function logoutUrl(cfg: OidcConfig): string {
  switch (cfg.provider) {
    case 'auth0':
      return `https://${cfg.auth0Domain}/v2/logout`
    case 'okta':
      return `https://${cfg.oktaDomain}/oauth2/${cfg.oktaAuthServerId}/v1/logout`
    default:
      return `${cfg.keycloakUrl}/realms/${cfg.realm}/protocol/openid-connect/logout`
  }
}

function discoveryUrl(cfg: OidcConfig): string {
  switch (cfg.provider) {
    case 'auth0':
      return `https://${cfg.auth0Domain}/.well-known/openid-configuration`
    case 'okta':
      return `https://${cfg.oktaDomain}/oauth2/${cfg.oktaAuthServerId}/.well-known/openid-configuration`
    default:
      return `${cfg.keycloakUrl}/realms/${cfg.realm}/.well-known/openid-configuration`
  }
}

export const authService = {
  async login(username: string, password: string) {
    const cfg = getOidcConfig()
    const url = tokenUrl(cfg)

    const params = new URLSearchParams({
      grant_type: 'password',
      username,
      password,
      scope: 'openid profile email',
    })

    switch (cfg.provider) {
      case 'auth0':
        params.append('client_id', cfg.clientId || cfg.auth0Domain)
        params.append('client_secret', cfg.clientSecret)
        params.append('audience', cfg.auth0Audience)
        break
      case 'okta':
        params.append('client_id', cfg.clientId)
        params.append('client_secret', cfg.clientSecret)
        break
      default:
        params.append('client_id', cfg.clientId)
        if (cfg.clientSecret) params.append('client_secret', cfg.clientSecret)
    }

    const response = await axios.post(url, params, {
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    })
    return response.data
  },

  async refreshToken(refreshToken: string) {
    const cfg = getOidcConfig()
    const url = tokenUrl(cfg)

    const params = new URLSearchParams({
      grant_type: 'refresh_token',
      refresh_token: refreshToken,
    })

    switch (cfg.provider) {
      case 'auth0':
        params.append('client_id', cfg.clientId)
        params.append('client_secret', cfg.clientSecret)
        break
      case 'okta':
        params.append('client_id', cfg.clientId)
        params.append('client_secret', cfg.clientSecret)
        break
      default:
        params.append('client_id', cfg.clientId)
        if (cfg.clientSecret) params.append('client_secret', cfg.clientSecret)
    }

    const response = await axios.post(url, params, {
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    })
    return response.data
  },

  async getUserInfo(token: string) {
    const cfg = getOidcConfig()
    const response = await axios.get(userInfoUrl(cfg), {
      headers: { Authorization: `Bearer ${token}` },
    })
    return response.data
  },

  async validateWithControlPlane(token: string) {
    const cfg = getOidcConfig()
    const response = await axios.get(`${cfg.controlPlaneUrl}/users/me`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    return response.data
  },

  async logout(_token: string, refreshToken: string) {
    const cfg = getOidcConfig()
    const url = logoutUrl(cfg)

    if (cfg.provider === 'auth0') {
      // Auth0 uses a redirect-based logout; best-effort token revoke
      const params = new URLSearchParams({
        client_id: cfg.clientId,
        token: refreshToken,
      })
      await axios.post(`https://${cfg.auth0Domain}/oauth/revoke`, params, {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      }).catch(() => {})
      return
    }

    const params = new URLSearchParams({
      client_id: cfg.clientId,
      refresh_token: refreshToken,
    })
    if (cfg.clientSecret) params.append('client_secret', cfg.clientSecret)

    await axios.post(url, params, {
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    })
  },

  async testConnection(cfg: OidcConfig): Promise<void> {
    await axios.get(discoveryUrl(cfg), { timeout: 8000 })
  },
}
