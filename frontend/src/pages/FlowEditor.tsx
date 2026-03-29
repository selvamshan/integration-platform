import { useState, useEffect } from 'react'
import { useParams, Link } from 'react-router-dom'
import { ArrowLeft } from 'lucide-react'
import { FlowDesigner } from '@/components/Flows/FlowDesigner'
import { flowService } from '@/services/flow'
import { Flow } from '@/types/flow'

export function FlowEditor() {
  const { id } = useParams<{ id: string }>()
  const isNew = id === 'new'
  const [flow, setFlow] = useState<Flow | undefined>(undefined)
  const [loading, setLoading] = useState(!isNew)
  const [loadError, setLoadError] = useState<string | null>(null)

  useEffect(() => {
    if (!isNew && id) {
      flowService.get(id)
        .then(setFlow)
        .catch((err) => setLoadError(err?.response?.data?.error ?? err?.message ?? 'Failed to load flow'))
        .finally(() => setLoading(false))
    }
  }, [id, isNew])

  return (
    <div>
      <div className="flex items-center gap-4 mb-6">
        <Link to="/flows" className="text-gray-500 hover:text-gray-800">
          <ArrowLeft className="w-5 h-5" />
        </Link>
        <h1 className="text-3xl font-bold">{isNew ? 'Create Flow' : 'Edit Flow'}</h1>
      </div>

      {loading && (
        <div className="flex items-center justify-center h-64 text-gray-500">Loading flow…</div>
      )}
      {loadError && (
        <div className="text-red-600 p-4 border border-red-200 rounded-lg">{loadError}</div>
      )}
      {!loading && !loadError && (
        <FlowDesigner flowId={isNew ? undefined : id} initialFlow={flow} />
      )}
    </div>
  )
}
