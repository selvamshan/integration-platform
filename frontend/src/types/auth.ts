export interface User {
  sub: string
  preferred_username: string
  email?: string
  roles?: string[]
}

export interface AuthTokens {
  access_token: string
  refresh_token: string
}
