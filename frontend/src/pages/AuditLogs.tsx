import { useState, useEffect, useCallback } from 'react'
import { useSearchParams, Link } from 'react-router-dom'
import { auditService, AuditLog } from '@/services/audit'

const ACTION_COLORS: Record<string, string> = {
  Create:     'bg-blue-100 text-blue-700',
  Update:     'bg-yellow-100 text-yellow-700',
  Delete:     'bg-red-100 text-red-700',
  Execute:    'bg-purple-100 text-purple-700',
  Enable:     'bg-green-100 text-green-700',
  Disable:    'bg-gray-100 text-gray-600',
  Test:       'bg-indigo-100 text-indigo-700',
  Schedule:   'bg-teal-100 text-teal-700',
  Unschedule: 'bg-orange-100 text-orange-700',
}

const ENTITY_LABELS: Record<string, string> = {
  flow:               'Flow',
  connector_instance: 'Connector',
}

function Badge({ text, colorClass }: { text: string; colorClass: string }) {
  return (
    <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${colorClass}`}>
      {text}
    </span>
  )
}

function formatDuration(ms: number | null) {
  if (ms === null) return '—'
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function formatTime(iso: string) {
  const d = new Date(iso)
  return d.toLocaleString(undefined, {
    month: 'short', day: 'numeric',
    hour: '2-digit', minute: '2-digit', second: '2-digit',
  })
}

const TABS = [
  { label: 'All',        value: '' },
  { label: 'Flows',      value: 'flow' },
  { label: 'Connectors', value: 'connector_instance' },
]

export function AuditLogs() {
  const [searchParams, setSearchParams] = useSearchParams()

  const [logs, setLogs] = useState<AuditLog[]>([])
  const [count, setCount] = useState(0)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const [entityType, setEntityType] = useState(searchParams.get('entity_type') ?? '')
  const [entityId, setEntityId]     = useState(searchParams.get('entity_id') ?? '')
  const [userId, setUserId]         = useState(searchParams.get('user_id') ?? '')
  const [limit, setLimit]           = useState(Number(searchParams.get('limit') ?? 50))

  const applyTab = (value: string) => {
    setEntityType(value)
    setEntityId('')
  }

  const fetchLogs = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const params: Record<string, any> = { limit }
      if (entityType) params.entity_type = entityType
      if (entityId)   params.entity_id   = entityId
      if (userId)     params.user_id     = userId

      let data
      if (entityType === 'flow' && entityId) {
        data = await auditService.getFlowLogs(entityId)
      } else if (entityType === 'connector_instance' && entityId) {
        data = await auditService.getConnectorLogs(entityId)
      } else {
        data = await auditService.list(params)
      }
      setLogs(data.logs)
      setCount(data.count)
    } catch (e: any) {
      setError(e?.response?.data?.error ?? 'Failed to load audit logs')
    } finally {
      setLoading(false)
    }
  }, [entityType, entityId, userId, limit])

  useEffect(() => {
    fetchLogs()
  }, [])  // load on mount with initial params

  const handleSearch = () => {
    const p: Record<string, string> = {}
    if (entityType) p.entity_type = entityType
    if (entityId)   p.entity_id   = entityId
    if (userId)     p.user_id     = userId
    if (limit !== 50) p.limit = String(limit)
    setSearchParams(p)
    fetchLogs()
  }

  const handleClear = () => {
    setEntityType('')
    setEntityId('')
    setUserId('')
    setLimit(50)
    setSearchParams({})
  }

  const entityLabel = entityType ? (ENTITY_LABELS[entityType] ?? entityType) : null

  return (
    <div>
      <div className="flex justify-between items-center mb-4">
        <div>
          <h1 className="text-3xl font-bold">Audit Logs</h1>
          {entityLabel && entityId && (
            <p className="text-sm text-gray-500 mt-1">
              Showing logs for {entityLabel} <span className="font-mono">{entityId}</span>
            </p>
          )}
        </div>
        <span className="text-sm text-gray-500">{count} record{count !== 1 ? 's' : ''}</span>
      </div>

      {/* Quick-filter tabs */}
      <div className="flex gap-1 mb-5 border-b border-gray-200">
        {TABS.map((tab) => (
          <button
            key={tab.value}
            onClick={() => applyTab(tab.value)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors -mb-px ${
              entityType === tab.value
                ? 'border-primary-600 text-primary-600'
                : 'border-transparent text-gray-500 hover:text-gray-800'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Filters */}
      <div className="card mb-6">
        <div className="flex flex-wrap gap-3 items-end">
          <div className="flex-1 min-w-[140px]">
            <label className="block text-sm font-medium text-gray-700 mb-1">Entity Type</label>
            <select
              value={entityType}
              onChange={(e) => setEntityType(e.target.value)}
              className="input"
            >
              <option value="">All</option>
              <option value="flow">Flow</option>
              <option value="connector_instance">Connector</option>
            </select>
          </div>
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-gray-700 mb-1">Entity ID</label>
            <input
              type="text"
              placeholder="UUID"
              value={entityId}
              onChange={(e) => setEntityId(e.target.value)}
              className="input font-mono text-sm"
            />
          </div>
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-gray-700 mb-1">User ID</label>
            <input
              type="text"
              placeholder="User ID"
              value={userId}
              onChange={(e) => setUserId(e.target.value)}
              className="input text-sm"
            />
          </div>
          <div className="w-24">
            <label className="block text-sm font-medium text-gray-700 mb-1">Limit</label>
            <select
              value={limit}
              onChange={(e) => setLimit(Number(e.target.value))}
              className="input"
            >
              {[25, 50, 100, 200].map((n) => (
                <option key={n} value={n}>{n}</option>
              ))}
            </select>
          </div>
          <div className="flex gap-2 pb-0.5">
            <button onClick={handleSearch} className="btn btn-primary" disabled={loading}>
              {loading ? 'Loading…' : 'Search'}
            </button>
            <button onClick={handleClear} className="btn btn-secondary">
              Clear
            </button>
          </div>
        </div>
      </div>

      {error && (
        <div className="card mb-4 border-red-200 bg-red-50 text-red-700 text-sm">{error}</div>
      )}

      {/* Table */}
      {logs.length === 0 && !loading ? (
        <p className="text-gray-500 text-center py-12">No audit logs found.</p>
      ) : (
        <div className="card p-0 overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b text-left text-gray-500 text-xs uppercase tracking-wide">
                <th className="px-4 py-3">Time</th>
                <th className="px-4 py-3">Entity</th>
                <th className="px-4 py-3">Action</th>
                <th className="px-4 py-3">Status</th>
                <th className="px-4 py-3">User</th>
                <th className="px-4 py-3">Duration</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {logs.map((log) => (
                <tr key={log.id} className="hover:bg-gray-50">
                  <td className="px-4 py-3 text-gray-500 whitespace-nowrap">
                    {formatTime(log.created_at)}
                  </td>
                  <td className="px-4 py-3">
                    <div className="flex items-center gap-1.5">
                      <span className="text-xs text-gray-400">
                        {ENTITY_LABELS[log.entity_type] ?? log.entity_type}
                      </span>
                      {log.entity_name && (
                        <span className="font-medium">{log.entity_name}</span>
                      )}
                    </div>
                    <EntityLink entityType={log.entity_type} entityId={log.entity_id} />
                  </td>
                  <td className="px-4 py-3">
                    <Badge
                      text={log.action}
                      colorClass={ACTION_COLORS[log.action] ?? 'bg-gray-100 text-gray-600'}
                    />
                  </td>
                  <td className="px-4 py-3">
                    <Badge
                      text={log.status}
                      colorClass={
                        log.status === 'Success'
                          ? 'bg-green-100 text-green-700'
                          : 'bg-red-100 text-red-700'
                      }
                    />
                    {log.error_message && (
                      <p className="text-xs text-red-600 mt-1 max-w-xs truncate" title={log.error_message}>
                        {log.error_message}
                      </p>
                    )}
                  </td>
                  <td className="px-4 py-3">
                    <div>{log.user_email ?? log.user_id}</div>
                    {log.user_role && (
                      <div className="text-xs text-gray-400">{log.user_role}</div>
                    )}
                  </td>
                  <td className="px-4 py-3 text-gray-500 whitespace-nowrap">
                    {formatDuration(log.duration_ms)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}

function EntityLink({ entityType, entityId }: { entityType: string; entityId: string }) {
  const short = entityId.slice(0, 8) + '…'
  if (entityType === 'flow') {
    return (
      <Link
        to={`/flows/${entityId}/runs`}
        className="text-xs text-primary-600 hover:underline font-mono"
      >
        {short}
      </Link>
    )
  }
  if (entityType === 'connector_instance') {
    return (
      <Link
        to={`/connectors`}
        className="text-xs text-primary-600 hover:underline font-mono"
      >
        {short}
      </Link>
    )
  }
  return <span className="text-xs text-gray-400 font-mono">{short}</span>
}
