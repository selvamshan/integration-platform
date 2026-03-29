import { useState, useEffect } from 'react'
import { connectorService } from '@/services/connector'
import { ConnectorInstance, Connector } from '@/types/connector'
import { ConnectorList } from '@/components/Connectors/ConnectorList'
import { ConnectorForm } from '@/components/Connectors/ConnectorForm'

export function Connectors() {
  const [connectors, setConnectors] = useState<ConnectorInstance[]>([])
  const [showCreateForm, setShowCreateForm] = useState(false)
  const [editingConnector, setEditingConnector] = useState<ConnectorInstance | null>(null)

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
    setShowCreateForm(false)
    loadConnectors()
  }

  const handleEdit = (connector: ConnectorInstance) => {
    setShowCreateForm(false)
    setEditingConnector(connector)
  }

  const handleUpdate = async (data: Connector) => {
    if (!editingConnector) return
    await connectorService.update(editingConnector.id, data)
    setEditingConnector(null)
    loadConnectors()
  }

  const handleDelete = async (id: string) => {
    if (!window.confirm('Delete this connector?')) return
    if (editingConnector?.id === id) setEditingConnector(null)
    try {
      await connectorService.delete(id)
      loadConnectors()
    } catch (err) {
      console.error('Failed to delete connector:', err)
    }
  }

  return (
    <div>
      <div className="flex justify-between mb-6">
        <h1 className="text-3xl font-bold">Connectors</h1>
        <button
          onClick={() => { setShowCreateForm(!showCreateForm); setEditingConnector(null) }}
          className="btn btn-primary"
        >
          {showCreateForm ? 'Cancel' : 'Create Connector'}
        </button>
      </div>

      {showCreateForm && (
        <div className="card mb-6">
          <h2 className="text-xl font-bold mb-4">New Connector</h2>
          <ConnectorForm onSubmit={handleCreate} />
        </div>
      )}

      {editingConnector && (
        <div className="card mb-6">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-xl font-bold">Edit Connector</h2>
            <button onClick={() => setEditingConnector(null)} className="text-sm text-gray-500 hover:text-gray-700">
              Cancel
            </button>
          </div>
          <ConnectorForm
            onSubmit={handleUpdate}
            initialValues={editingConnector as Connector}
            isEdit
          />
        </div>
      )}

      <ConnectorList connectors={connectors} onEdit={handleEdit} onDelete={handleDelete} />
    </div>
  )
}
