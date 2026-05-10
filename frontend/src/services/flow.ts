import { api } from './api'
import { Flow } from '@/types/flow'

export interface TestStepResult {
  name: string
  step_type: string
  success: boolean
  output?: any
  error?: string
  duration_ms: number
}

export interface TestFlowResult {
  success: boolean
  result: any
  error: string | null
  execution: {
    duration_ms: number
    steps_executed: number
    step_results: TestStepResult[]
    output?: any
  }
}

export interface NodeRunResult {
  node_id: string
  success: boolean
  error?: string
  duration_ms: number
  output?: any
}

export interface FlowRunRecord {
  run_id: string
  flow_id: string
  flow_name: string
  started_at: string
  duration_ms: number
  success: boolean
  error?: string
  node_results: NodeRunResult[]
}

export const flowService = {
  async list() {
    const res = await api.get<{ flows: Flow[] }>('/flows')
    return res.data
  },

  async get(id: string) {
    const res = await api.get<Flow>(`/flows/${id}`)
    return res.data
  },

  async create(flow: Omit<Flow, 'active'>) {
    const res = await api.post('/flows', flow)
    return res.data
  },

  async update(id: string, flow: Omit<Flow, 'active'>) {
    const res = await api.put(`/flows/${id}`, flow)
    return res.data
  },

  async delete(id: string) {
    await api.delete(`/flows/${id}`)
  },

  async test(flow: Omit<Flow, 'active'>, testInput?: any): Promise<TestFlowResult> {
    const res = await api.post<TestFlowResult>('/flows/test', { flow, test_input: testInput ?? {} })
    return res.data
  },

  async getRuns(flowId: string): Promise<{ runs: FlowRunRecord[] }> {
    const dataplaneUrl = import.meta.env.VITE_DATA_PLANE_URL || 'http://localhost:8080'
    const res = await fetch(`${dataplaneUrl}/flows/${flowId}/runs`)
    if (!res.ok) throw new Error(`Data-plane returned ${res.status}`)
    return res.json()
  },
}
