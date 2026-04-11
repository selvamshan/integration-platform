import axios from 'axios'

const KEYCLOAK_URL = import.meta.env.VITE_KEYCLOAK_URL || '/keycloak'
const REALM = import.meta.env.VITE_KEYCLOAK_REALM || 'integration-platform'
const CLIENT_ID = import.meta.env.VITE_KEYCLOAK_CLIENT_ID || 'control-plane'
const CLIENT_SECRET = import.meta.env.VITE_KEYCLOAK_CLIENT_SECRET || ''

export const authService = {
  /**
   * Login with Keycloak using Resource Owner Password Credentials flow
   */
  async login(username: string, password: string) {
    const tokenUrl = `${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/token`
    
    const params = new URLSearchParams({
      client_id: CLIENT_ID,
      grant_type: 'password',
      username,
      password,
      scope: 'openid profile email',
    })

    // Add client_secret if configured
    if (CLIENT_SECRET) {
      params.append('client_secret', CLIENT_SECRET)
    }

    const response = await axios.post(tokenUrl, params, {
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
    })
    console.log(response.data)
    return response.data
  },

  /**
   * Refresh access token using refresh token
   */
  async refreshToken(refreshToken: string) {
    const tokenUrl = `${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/token`
    
    const params = new URLSearchParams({
      client_id: CLIENT_ID,
      grant_type: 'refresh_token',
      refresh_token: refreshToken,
    })

    if (CLIENT_SECRET) {
      params.append('client_secret', CLIENT_SECRET)
    }

    const response = await axios.post(tokenUrl, params, {
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
    })

    return response.data
  },

  /**
   * Get user info from Keycloak
   */
  async getUserInfo(token: string) {
    const userInfoUrl = `${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/userinfo`
    
    const response = await axios.get(userInfoUrl, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    })

    return response.data
  },

  /**
   * Validate token with Control Plane
   */
  async validateWithControlPlane(token: string) {
    const controlPlaneUrl = import.meta.env.VITE_CONTROL_PLANE_URL || 'http://localhost:8081'
    
    const response = await axios.get(`${controlPlaneUrl}/users/me`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    })

    return response.data
  },

  /**
   * Logout (revoke token)
   */
  async logout(token: string, refreshToken: string) {
    const logoutUrl = `${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/logout`
    
    const params = new URLSearchParams({
      client_id: CLIENT_ID,
      refresh_token: refreshToken,
    })

    if (CLIENT_SECRET) {
      params.append('client_secret', CLIENT_SECRET)
    }

    await axios.post(logoutUrl, params, {
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
    })
  },
}
