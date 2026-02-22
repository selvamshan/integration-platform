import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { flowService } from '@/services/flow'
import { Flow } from '@/types/flow'

export function Flows() {
  const [flows, setFlows] = useState<Flow[]>([])
  
  useEffect(() => {
    flowService.list().then((d) => setFlows(d.flows))
  }, [])
  
  return (
    <div>
      <div className="flex justify-between mb-6">
        <h1 className="text-3xl font-bold">Flows</h1>
        <Link to="/flows/new" className="btn btn-primary">Create Flow</Link>
      </div>
      <div className="grid gap-4">
        {flows.map((f) => (
          <div key={f.id} className="card">
            <h3 className="font-bold">{f.name}</h3>
            <p className="text-sm text-gray-600">{f.trigger.method} {f.trigger.path}</p>
          </div>
        ))}
      </div>
    </div>
  )
}
