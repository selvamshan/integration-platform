import { api } from './api'

export interface TriggerDefinition {
  id: string
  name: string
  trigger_type: string
  description: string
  icon: string
  config_schema: any
  enabled: boolean
}

export const triggerDefinitionService = {
  /**
   * Get all available trigger types
   * Uses GET /triggers endpoint
   */
  async list(): Promise<{ triggers: TriggerDefinition[], count: number }> {
    const response = await api.get('/triggers')
    return response.data
  },
}
