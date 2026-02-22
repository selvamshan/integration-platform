import { api } from './api'
import { Connector, ConnectorInstance } from '@/types/connector'

export const connectorService = {
  async list() {
    const res = await api.get<{ connectors: ConnectorInstance[] }>('/connector-instances')
    return res.data
  },
  
  async create(connector: Connector) {
    const res = await api.post('/connector-instances', connector)
    return res.data
  },
  
  async delete(id: string) {
    await api.delete(`/connector-instances/${id}`)
  },
}
