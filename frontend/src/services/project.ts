import { api } from './api'
import { Project } from '@/types/project'
import { Flow } from '@/types/flow'

export const projectService = {
  async list() {
    const res = await api.get<{ projects: Project[]; count: number }>('/projects')
    return res.data
  },

  async get(id: string) {
    const res = await api.get<Project>(`/projects/${id}`)
    return res.data
  },

  async create(data: { name: string; description?: string }) {
    const res = await api.post<Project>('/projects', data)
    return res.data
  },

  async delete(id: string) {
    await api.delete(`/projects/${id}`)
  },

  async listFlows(id: string) {
    const res = await api.get<{ flows: Flow[]; count: number }>(`/projects/${id}/flows`)
    return res.data
  },
}
