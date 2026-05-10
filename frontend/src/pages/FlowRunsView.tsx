import { useState, useEffect, useCallback } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { ArrowLeft, Pencil, Trash2, Play, RefreshCw, X, CheckCircle2, XCircle, Circle, AlertTriangle } from 'lucide-react'
import { flowService, FlowRunRecord, NodeRunResult } from '@/services/flow'
import { Flow } from '@/types/flow'

// ── Helpers ────────────────────────────────────────────────────────────────────

function formatTime(iso: string) {
  const d = new Date(iso)
  return d.toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}

function formatDuration(ms: number) {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(2)}s`
}

function relativeTime(iso: string) {
  const diff = Date.now() - new Date(iso).getTime()
  const secs = Math.floor(diff / 1000)
  if (secs < 60) return `${secs}s ago`
  const mins = Math.floor(secs / 60)
  if (mins < 60) return `${mins}m ago`
  const hrs = Math.floor(mins / 60)
  if (hrs < 24) return `${hrs}h ago`
  return `${Math.floor(hrs / 24)}d ago`
}

// ── Status cell ────────────────────────────────────────────────────────────────

type NodeStatus = 'success' | 'failed' | 'skipped'

function statusForNode(run: FlowRunRecord, nodeId: string): NodeStatus {
  const nr = run.node_results.find((r) => r.node_id === nodeId)
  if (!nr) return 'skipped'
  return nr.success ? 'success' : 'failed'
}

function StatusCell({ status, onClick }: { status: NodeStatus; onClick: () => void }) {
  const base = 'w-8 h-8 rounded flex items-center justify-center cursor-pointer transition-transform hover:scale-110 hover:ring-2 hover:ring-offset-1'
  if (status === 'success')
    return (
      <button onClick={onClick} className={`${base} bg-green-500 hover:ring-green-400`} title="Success">
        <CheckCircle2 className="w-4 h-4 text-white" />
      </button>
    )
  if (status === 'failed')
    return (
      <button onClick={onClick} className={`${base} bg-red-500 hover:ring-red-400`} title="Failed">
        <XCircle className="w-4 h-4 text-white" />
      </button>
    )
  return (
    <button onClick={onClick} className={`${base} bg-gray-200 hover:ring-gray-300`} title="Not run / skipped">
      <Circle className="w-4 h-4 text-gray-400" />
    </button>
  )
}

// ── Log panel ──────────────────────────────────────────────────────────────────

interface LogSelection {
  run: FlowRunRecord
  nodeId: string
  nodeResult: NodeRunResult | null
}

function LogPanel({ sel, onClose }: { sel: LogSelection; onClose: () => void }) {
  const nr = sel.nodeResult
  return (
    <div className="fixed inset-y-0 right-0 w-[440px] bg-white shadow-2xl border-l border-gray-200 flex flex-col z-50">
      <div className="flex items-center justify-between px-4 py-3 border-b bg-gray-50">
        <div>
          <p className="font-semibold text-sm text-gray-900 font-mono">{sel.nodeId}</p>
          <p className="text-xs text-gray-500">{formatTime(sel.run.started_at)} · {relativeTime(sel.run.started_at)}</p>
        </div>
        <button onClick={onClose} className="p-1.5 rounded hover:bg-gray-200">
          <X className="w-4 h-4" />
        </button>
      </div>

      <div className="flex-1 overflow-auto p-4 space-y-4 text-sm">
        {/* Run summary */}
        <div className={`rounded-lg p-3 border ${sel.run.success ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'}`}>
          <p className="font-medium mb-1">{sel.run.success ? '✅ Run succeeded' : '❌ Run failed'}</p>
          <p className="text-xs text-gray-600">Total duration: {formatDuration(sel.run.duration_ms)}</p>
          {sel.run.error && <p className="text-xs text-red-600 mt-1">{sel.run.error}</p>}
        </div>

        {/* Node-level detail */}
        {nr ? (
          <>
            <div className={`rounded-lg p-3 border ${nr.success ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'}`}>
              <p className="font-medium mb-1">{nr.success ? '✅ Node succeeded' : '❌ Node failed'}</p>
              <p className="text-xs text-gray-600">Node duration: {formatDuration(nr.duration_ms)}</p>
              {nr.error && (
                <div className="mt-2">
                  <p className="text-xs font-semibold text-red-700 mb-1">Error</p>
                  <pre className="text-xs font-mono text-red-800 bg-red-100 rounded p-2 overflow-auto whitespace-pre-wrap">{nr.error}</pre>
                </div>
              )}
            </div>

            {nr.output !== undefined && nr.output !== null && (
              <div>
                <p className="text-xs font-semibold text-gray-600 mb-1.5 uppercase tracking-wide">Backend Output / Logs</p>
                <pre className="text-xs font-mono bg-gray-900 text-green-400 rounded-lg p-3 overflow-auto max-h-96 whitespace-pre-wrap leading-relaxed">
                  {JSON.stringify(nr.output, null, 2)}
                </pre>
              </div>
            )}
          </>
        ) : (
          <div className="rounded-lg p-3 bg-gray-50 border border-gray-200 text-gray-500 text-sm">
            This node was not reached in this run (skipped due to edge conditions or an upstream failure).
          </div>
        )}

        <div>
          <p className="text-xs font-semibold text-gray-600 mb-1.5 uppercase tracking-wide">Run Details</p>
          <table className="text-xs w-full">
            <tbody className="divide-y divide-gray-100">
              <tr><td className="py-1.5 text-gray-500 pr-3">Run ID</td><td className="py-1.5 font-mono text-gray-700 break-all">{sel.run.run_id}</td></tr>
              <tr><td className="py-1.5 text-gray-500 pr-3">Flow</td><td className="py-1.5">{sel.run.flow_name}</td></tr>
              <tr><td className="py-1.5 text-gray-500 pr-3">Started</td><td className="py-1.5">{formatTime(sel.run.started_at)}</td></tr>
              <tr><td className="py-1.5 text-gray-500 pr-3">Total duration</td><td className="py-1.5">{formatDuration(sel.run.duration_ms)}</td></tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}

// ── Main page ──────────────────────────────────────────────────────────────────

export function FlowRunsView() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const [flow, setFlow] = useState<Flow | null>(null)
  const [runs, setRuns] = useState<FlowRunRecord[]>([])
  const [loading, setLoading] = useState(true)
  const [runsError, setRunsError] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [deletingId, setDeletingId] = useState(false)
  const [logSel, setLogSel] = useState<LogSelection | null>(null)
  const [triggering, setTriggering] = useState(false)
  const [triggerResult, setTriggerResult] = useState<{ ok: boolean; msg: string } | null>(null)

  const fetchRuns = useCallback(async (flowId: string) => {
    setRunsError(null)
    try {
      const data = await flowService.getRuns(flowId)
      setRuns(data.runs)
    } catch (e: any) {
      setRunsError('Could not reach data-plane. Make sure it is running.')
    }
  }, [])

  const load = useCallback(async () => {
    if (!id) return
    setLoading(true)
    setError(null)
    try {
      const flowData = await flowService.get(id)
      setFlow(flowData)
      await fetchRuns(id)
    } catch (e: any) {
      setError(e?.message ?? 'Failed to load flow')
    } finally {
      setLoading(false)
    }
  }, [id, fetchRuns])

  useEffect(() => { load() }, [load])

  const handleDelete = async () => {
    if (!flow || !id) return
    if (!confirm(`Delete flow "${flow.name}"?`)) return
    setDeletingId(true)
    try {
      await flowService.delete(id)
      navigate('/projects')
    } finally {
      setDeletingId(false)
    }
  }

  const handleTrigger = async () => {
    if (!flow || !id) return
    setTriggering(true)
    setTriggerResult(null)
    const dataplaneUrl = import.meta.env.VITE_DATA_PLANE_URL || 'http://localhost:8080'

    try {
      if (flow.trigger.type === 'http') {
        const trigger = flow.trigger as any
        const url = `${dataplaneUrl}/api/trigger${trigger.path}`
        const res = await fetch(url, {
          method: trigger.method ?? 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: ['GET', 'HEAD'].includes((trigger.method ?? 'POST').toUpperCase()) ? undefined : '{}',
        })
        const text = await res.text()
        setTriggerResult({
          ok: res.ok,
          msg: res.ok ? `Triggered — ${res.status} ${res.statusText}` : `${res.status}: ${text}`,
        })
      } else {
        // Schedule flow: use the execute endpoint
        const res = await fetch(`${dataplaneUrl}/flows/${flow.name}/execute`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({}),
        })
        setTriggerResult({ ok: res.ok, msg: res.ok ? 'Flow executed' : `Error ${res.status}` })
      }
      // Refresh runs after a short delay
      setTimeout(() => fetchRuns(id), 800)
    } catch (e: any) {
      setTriggerResult({ ok: false, msg: e?.message ?? 'Failed to trigger flow — is the data-plane running?' })
    } finally {
      setTriggering(false)
    }
  }

  // All node IDs ordered by position in the flow definition
  const nodeIds: string[] = flow ? flow.nodes.map((n) => n.id) : []
  const runNodeIds = new Set(runs.flatMap((r) => r.node_results.map((nr) => nr.node_id)))
  const allNodeIds = [...nodeIds, ...[...runNodeIds].filter((nid) => !nodeIds.includes(nid))]

  const triggerLabel =
    flow?.trigger.type === 'http'
      ? `${(flow.trigger as any).method} ${(flow.trigger as any).path}`
      : flow?.trigger.type === 'schedule'
      ? `schedule: ${(flow.trigger as any).cron}`
      : ''

  const openLog = (run: FlowRunRecord, nodeId: string) => {
    const nodeResult = run.node_results.find((r) => r.node_id === nodeId) ?? null
    setLogSel({ run, nodeId, nodeResult })
  }

  if (loading) {
    return <div className="flex items-center justify-center h-64 text-gray-500">Loading…</div>
  }

  if (error || !flow) {
    return (
      <div className="text-red-600 p-4 border border-red-200 rounded-lg">{error ?? 'Flow not found'}</div>
    )
  }

  return (
    <div className="relative">
      {/* Header */}
      <div className="flex items-start justify-between mb-6">
        <div className="flex items-center gap-3">
          <button onClick={() => navigate('/projects')} className="text-gray-400 hover:text-gray-700 mt-0.5">
            <ArrowLeft className="w-5 h-5" />
          </button>
          <div>
            <h1 className="text-2xl font-bold text-gray-900">{flow.name}</h1>
            <p className="text-sm text-gray-500 font-mono mt-0.5">{triggerLabel}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => navigate(`/flows/${id}`)}
            className="btn btn-secondary flex items-center gap-1.5 text-sm"
          >
            <Pencil className="w-4 h-4" />
            Edit
          </button>
          <button
            onClick={handleDelete}
            disabled={deletingId}
            className="btn btn-secondary flex items-center gap-1.5 text-sm text-red-600 hover:bg-red-50 disabled:opacity-50"
          >
            <Trash2 className="w-4 h-4" />
            {deletingId ? 'Deleting…' : 'Delete'}
          </button>
          <button
            onClick={handleTrigger}
            disabled={triggering}
            className="btn btn-primary flex items-center gap-1.5 text-sm disabled:opacity-60"
          >
            <Play className="w-4 h-4" />
            {triggering ? 'Running…' : 'Run Now'}
          </button>
        </div>
      </div>

      {/* Trigger result banner */}
      {triggerResult && (
        <div className={`mb-4 flex items-center justify-between px-4 py-2.5 rounded-lg text-sm ${
          triggerResult.ok ? 'bg-green-50 border border-green-200 text-green-800' : 'bg-red-50 border border-red-200 text-red-800'
        }`}>
          <span>{triggerResult.ok ? '✅' : '❌'} {triggerResult.msg}</span>
          <button onClick={() => setTriggerResult(null)} className="ml-4 text-gray-400 hover:text-gray-600">
            <X className="w-4 h-4" />
          </button>
        </div>
      )}

      {/* Runs grid */}
      <div className="card p-0 overflow-hidden">
        <div className="flex items-center justify-between px-5 py-3 border-b bg-gray-50">
          <div>
            <h2 className="font-semibold text-sm text-gray-900">Last Runs</h2>
            <p className="text-xs text-gray-500 mt-0.5">Node-wise execution history · click a cell to see logs</p>
          </div>
          <button
            onClick={() => id && fetchRuns(id)}
            disabled={loading}
            className="btn btn-secondary flex items-center gap-1.5 text-xs py-1.5"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </div>

        {/* Data-plane connection warning */}
        {runsError && (
          <div className="flex items-center gap-2 px-5 py-3 bg-amber-50 border-b border-amber-200 text-amber-800 text-sm">
            <AlertTriangle className="w-4 h-4 flex-shrink-0" />
            <span>{runsError}</span>
          </div>
        )}

        {runs.length === 0 ? (
          <div className="px-5 py-12 text-center">
            <p className="text-gray-500 text-sm mb-1">No runs recorded yet.</p>
            <p className="text-gray-400 text-xs">Click <strong>Run Now</strong> above to trigger this flow, then refresh.</p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="min-w-full text-sm">
              <thead>
                <tr className="border-b bg-gray-50">
                  <th className="text-left px-5 py-3 font-medium text-gray-600 text-xs uppercase tracking-wide w-52">
                    Node
                  </th>
                  {runs.map((run) => (
                    <th key={run.run_id} className="px-3 py-2 text-center min-w-[80px]">
                      <div className={`text-xs font-medium ${run.success ? 'text-green-700' : 'text-red-700'}`}>
                        {relativeTime(run.started_at)}
                      </div>
                      <div className="text-[10px] text-gray-400 mt-0.5">
                        {formatDuration(run.duration_ms)}
                      </div>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-50">
                {allNodeIds.map((nodeId) => (
                  <tr key={nodeId} className="hover:bg-gray-50/50">
                    <td className="px-5 py-2.5">
                      <span className="font-mono text-xs text-gray-800 truncate max-w-[192px] block" title={nodeId}>
                        {nodeId}
                      </span>
                    </td>
                    {runs.map((run) => {
                      const status = statusForNode(run, nodeId)
                      return (
                        <td key={run.run_id} className="px-3 py-2 text-center">
                          <div className="flex justify-center">
                            <StatusCell status={status} onClick={() => openLog(run, nodeId)} />
                          </div>
                        </td>
                      )
                    })}
                  </tr>
                ))}
              </tbody>

              {/* Overall run status row */}
              <tfoot>
                <tr className="border-t bg-gray-50">
                  <td className="px-5 py-2 text-xs font-medium text-gray-500 uppercase tracking-wide">
                    Overall
                  </td>
                  {runs.map((run) => (
                    <td key={run.run_id} className="px-3 py-2 text-center">
                      <span className={`inline-block w-2.5 h-2.5 rounded-full ${run.success ? 'bg-green-500' : 'bg-red-500'}`} />
                    </td>
                  ))}
                </tr>
              </tfoot>
            </table>
          </div>
        )}
      </div>

      {/* Log slide-over */}
      {logSel && (
        <>
          <div className="fixed inset-0 bg-black/20 z-40" onClick={() => setLogSel(null)} />
          <LogPanel sel={logSel} onClose={() => setLogSel(null)} />
        </>
      )}
    </div>
  )
}
