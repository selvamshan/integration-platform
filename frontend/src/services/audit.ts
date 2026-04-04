import { api } from './api'

export interface AuditLog {
  id: string
  entity_type: string
  entity_id: string
  entity_name: string | null
  action: string
  status: string
  user_id: string
  user_email: string | null
  user_role: string | null
  ip_address: string | null
  request_id: string | null
  error_message: string | null
  duration_ms: number | null
  created_at: string
}

export interface AuditLogsResponse {
  logs: AuditLog[]
  count: number
}

export interface AuditQueryParams {
  entity_type?: string
  entity_id?: string
  user_id?: string
  limit?: number
}

export const auditService = {
  async list(params?: AuditQueryParams): Promise<AuditLogsResponse> {
    const res = await api.get<AuditLogsResponse>('/audit-logs', { params })
    return res.data
  },

  async getFlowLogs(flowId: string): Promise<AuditLogsResponse> {
    const res = await api.get<AuditLogsResponse>(`/flows/${flowId}/audit-logs`)
    return res.data
  },

  async getConnectorLogs(connectorId: string): Promise<AuditLogsResponse> {
    const res = await api.get<AuditLogsResponse>(`/connector-instances/${connectorId}/audit-logs`)
    return res.data
  },
}
