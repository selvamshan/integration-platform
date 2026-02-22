export interface ConnectorInstance {
  id: string
  name: string
  connector_type: 'postgres' | 'mysql' | 'http'
  host?: string
  port?: number
  database_name?: string
  extra_attributes?: Record<string, any>
  active: boolean
}

export interface Connector {
  id: string
  name: string
  connector_type: string
  host?: string
  port?: number
  database_name?: string
  username?: string
  password?: string
  extra_attributes?: Record<string, any>
}
