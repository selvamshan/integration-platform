import { api } from './api'

export interface ConnectorDefinition {
  id: string
  name: string
  connector_type: string
  description: string
  icon: string
  operations: Operation[]
  config_schema: any
  enabled: boolean
}

export interface Operation {
  name: string
  description: string
  parameters: Parameter[]
}

export interface Parameter {
  name: string
  param_type: string
  required: boolean
  description: string
  default_value?: any
}

export const connectorDefinitionService = {
  /**
   * Get all available connector types
   * Uses GET /connectors endpoint
   */
  async list(): Promise<{ connectors: ConnectorDefinition[], count: number }> {
    const response = await api.get('/connectors')
    return response.data
  },
}
