import { api } from './api'
import { Flow } from '@/types/flow'

export const flowService = {
  async list() {
    const res = await api.get<{ flows: Flow[] }>('/flows')
    return res.data
  },
  
  async create(flow: Omit<Flow, 'active'>) {
    const res = await api.post('/flows', flow)
    return res.data
  },
  
  async delete(id: string) {
    await api.delete(`/flows/${id}`)
  },
}
