import { useState, useEffect } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { Pencil, Trash2 } from 'lucide-react'
import { flowService } from '@/services/flow'
import { Flow } from '@/types/flow'

export function Flows() {
  const [flows, setFlows] = useState<Flow[]>([])
  const [deletingId, setDeletingId] = useState<string | null>(null)
  const navigate = useNavigate()

  useEffect(() => {
    flowService.list().then((d) => setFlows(d.flows))
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
              <h3 className="font-bold">{f.name}</h3>
              <p className="text-sm text-gray-500 font-mono">{f.id}</p>
              <p className="text-sm text-gray-600 mt-0.5">{f.trigger.method} {f.trigger.path}</p>
            </div>
            <div className="flex items-center gap-2">
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
