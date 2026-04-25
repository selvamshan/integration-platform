import { api } from './api'
import { Connector, ConnectorInstance } from '@/types/connector'

export interface TestConnectionResult {
  success: boolean
  message: string
}

export const connectorService = {
  async list() {
    const res = await api.get<{ connectors: ConnectorInstance[] }>('/connector-instances')
    return res.data
  },

  async create(connector: Connector) {
    const res = await api.post('/connector-instances', connector)
    return res.data
  },

  async update(id: string, connector: Connector) {
    const res = await api.put(`/connector-instances/${id}`, connector)
    return res.data
  },

  async delete(id: string) {
    await api.delete(`/connector-instances/${id}`)
  },

  async testConnection(connector: Partial<Connector>): Promise<TestConnectionResult> {
    const res = await api.post<TestConnectionResult>('/connector-instances/test', connector)
    return res.data
  },
}
