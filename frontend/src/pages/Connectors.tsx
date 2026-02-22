import { useState, useEffect } from 'react'
import { connectorService } from '@/services/connector'
import { ConnectorInstance, Connector } from '@/types/connector'
import { ConnectorList } from '@/components/Connectors/ConnectorList'
import { ConnectorForm } from '@/components/Connectors/ConnectorForm'

export function Connectors() {
  const [connectors, setConnectors] = useState<ConnectorInstance[]>([])
  const [showForm, setShowForm] = useState(false)
  
  useEffect(() => {
    loadConnectors()
  }, [])
  
  const loadConnectors = async () => {
    try {
      const data = await connectorService.list()
      setConnectors(data.connectors ?? [])
    } catch (err) {
      console.error('Failed to load connectors:', err)
    }
  }
  
  const handleCreate = async (data: Connector) => {
    await connectorService.create(data)
    setShowForm(false)
    loadConnectors()
  }
  
  const handleDelete = async (id: string) => {
    await connectorService.delete(id)
    loadConnectors()
  }
  
  return (
    <div>
      <div className="flex justify-between mb-6">
        <h1 className="text-3xl font-bold">Connectors</h1>
        <button onClick={() => setShowForm(!showForm)} className="btn btn-primary">
          {showForm ? 'Cancel' : 'Create Connector'}
        </button>
      </div>
      
      {showForm && (
        <div className="card mb-6">
          <h2 className="text-xl font-bold mb-4">New Connector</h2>
          <ConnectorForm onSubmit={handleCreate} />
        </div>
      )}
      
      <ConnectorList connectors={connectors} onDelete={handleDelete} />
    </div>
  )
}
