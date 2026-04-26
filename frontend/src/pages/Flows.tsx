import { useState, useEffect } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { Pencil, Trash2, ScrollText } from 'lucide-react'
import { flowService } from '@/services/flow'
import { Flow } from '@/types/flow'
import { api } from '@/services/api'

interface Client {
  client_id: string
  name: string
}

export function Flows() {
  const [flows, setFlows] = useState<Flow[]>([])
  const [deletingId, setDeletingId] = useState<string | null>(null)
  const [clientMap, setClientMap] = useState<Record<string, string>>({})
  const navigate = useNavigate()

  useEffect(() => {
    flowService.list().then((d) => setFlows(d.flows))
    api.get('/auth/clients')
      .then((res) => {
        const map: Record<string, string> = {}
        for (const c of (res.data.clients ?? []) as Client[]) map[c.client_id] = c.name
        setClientMap(map)
      })
      .catch(() => {})
  }, [])

  const handleDelete = async (id: string, name: string) => {
    if (!confirm(`Delete flow "${name}"?`)) return
    setDeletingId(id)
    try {
      await flowService.delete(id)
      setFlows((prev) => prev.filter((f) => f.id !== id))
    } finally {
      setDeletingId(null)
    }
  }

  return (
    <div>
      <div className="flex justify-between mb-6">
        <h1 className="text-3xl font-bold">Flows</h1>
        <Link to="/flows/new" className="btn btn-primary">Create Flow</Link>
      </div>
      <div className="grid gap-4">
        {flows.map((f) => (
          <div key={f.id} className="card flex items-center justify-between">
            <div>
              <div className="flex items-center gap-2">
                <h3 className="font-bold">{f.name}</h3>
                {f.client_id && (
                  <span className="inline-flex items-center rounded-full bg-sky-100 px-2 py-0.5 text-xs font-medium text-sky-700">
                    {clientMap[f.client_id] ?? f.client_id}
                  </span>
                )}
              </div>
              <p className="text-sm text-gray-500 font-mono">{f.id}</p>
              <p className="text-sm text-gray-600 mt-0.5">
                {f.trigger.type === 'http'
                  ? `${f.trigger.method} ${f.trigger.path}`
                  : `schedule: ${f.trigger.cron}`}
              </p>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => navigate(`/audit-logs?entity_type=flow&entity_id=${f.id}`)}
                className="btn btn-secondary flex items-center gap-1.5 text-sm"
              >
                <ScrollText className="w-4 h-4" />
                Logs
              </button>
              <button
                onClick={() => navigate(`/flows/${f.id}`)}
                className="btn btn-secondary flex items-center gap-1.5 text-sm"
              >
                <Pencil className="w-4 h-4" />
                Edit
              </button>
              <button
                onClick={() => handleDelete(f.id, f.name)}
                disabled={deletingId === f.id}
                className="btn btn-secondary flex items-center gap-1.5 text-sm text-red-600 hover:bg-red-50 disabled:opacity-50"
              >
                <Trash2 className="w-4 h-4" />
                {deletingId === f.id ? 'Deleting…' : 'Delete'}
              </button>
            </div>
          </div>
        ))}
        {flows.length === 0 && (
          <p className="text-gray-500 text-center py-12">No flows yet. Create your first flow.</p>
        )}
      </div>
    </div>
  )
}
