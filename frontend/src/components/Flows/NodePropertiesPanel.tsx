import { useState, useEffect } from 'react'
import { Node } from 'reactflow'
import { X } from 'lucide-react'
import { api } from '@/services/api'

interface NodePropertiesPanelProps {
  selectedNode: Node | null
  onClose: () => void
  onUpdate: (nodeId: string, data: any) => void
}

export function NodePropertiesPanel({ 
  selectedNode, 
  onClose, 
  onUpdate 
}: NodePropertiesPanelProps) {
  const [properties, setProperties] = useState<any>({})
  const [connectorInstances, setConnectorInstances] = useState<any[]>([])
  const [selectedOperation, setSelectedOperation] = useState<any>(null)
  
  useEffect(() => {
    if (selectedNode) {
      setProperties(selectedNode.data.properties || {})
      
      // Load connector instances if it's a connector node
      if (selectedNode.data.type === 'connector') {
        const connectorType = selectedNode.data.definition?.connector_type
        if (connectorType) {
          api.get(`/connector-instances/type/${connectorType}`)
            .then(res => setConnectorInstances(res.data.instances || []))
            .catch(err => console.error('Failed to load connector instances:', err))
        }
      }
    }
  }, [selectedNode])

  // Update selected operation when operation changes
  useEffect(() => {
    if (selectedNode?.data.type === 'connector' && properties.operation) {
      const op = selectedNode.data.definition?.operations?.find(
        (o: any) => o.name === properties.operation
      )
      setSelectedOperation(op || null)
    }
  }, [properties.operation, selectedNode])
  
  if (!selectedNode) return null
  
  const handleChange = (field: string, value: any) => {
    const updated = { ...properties, [field]: value }
    setProperties(updated)
    onUpdate(selectedNode.id, { ...selectedNode.data, properties: updated })
  }

  const handleParamChange = (paramName: string, value: any) => {
    const updatedParams = { ...properties.params, [paramName]: value }
    handleChange('params', updatedParams)
  }
  
  const nodeType = selectedNode.data.type
  
  return (
    <div className="w-80 border-l bg-white overflow-y-auto">
      {/* Header */}
      <div className="p-4 border-b flex items-center justify-between sticky top-0 bg-white z-10">
        <h3 className="font-bold">Node Properties</h3>
        <button onClick={onClose} className="hover:bg-gray-100 p-1 rounded">
          <X className="w-4 h-4" />
        </button>
      </div>
      
      {/* Properties */}
      <div className="p-4 space-y-4">
        {/* Node Name */}
        <div>
          <label className="block text-sm font-medium mb-1">Node Name</label>
          <input
            type="text"
            value={selectedNode.data.label}
            onChange={(e) => onUpdate(selectedNode.id, { 
              ...selectedNode.data, 
              label: e.target.value 
            })}
            className="input"
          />
        </div>
        
        {/* HTTP Trigger Properties */}
        {nodeType === 'trigger' && (
          <>
            <div>
              <label className="block text-sm font-medium mb-1">Path</label>
              <input
                type="text"
                value={properties.path || ''}
                onChange={(e) => handleChange('path', e.target.value)}
                placeholder="/api/users"
                className="input"
              />
              <p className="text-xs text-gray-500 mt-1">e.g., /api/users or /webhook</p>
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Method</label>
              <select
                value={properties.method || 'POST'}
                onChange={(e) => handleChange('method', e.target.value)}
                className="input"
              >
                <option value="GET">GET</option>
                <option value="POST">POST</option>
                <option value="PUT">PUT</option>
                <option value="DELETE">DELETE</option>
              </select>
            </div>
          </>
        )}
        
        {/* Transform Properties */}
        {nodeType === 'transform' && (
          <>
            <div>
              <label className="block text-sm font-medium mb-1">Transform Type</label>
              <select
                value={properties.type || selectedNode.data.definition?.type || 'select'}
                onChange={(e) => handleChange('type', e.target.value)}
                className="input"
              >
                <option value="select">Select Fields</option>
                <option value="map">Map/Transform</option>
                <option value="filter">Filter</option>
                <option value="rename">Rename Fields</option>
                <option value="convert">Convert Types</option>
              </select>
            </div>
            
            {/* Dynamic fields based on transform type */}
            {(properties.type === 'select' || selectedNode.data.definition?.type === 'select') && (
              <div>
                <label className="block text-sm font-medium mb-1">
                  Fields to Select
                </label>
                <input
                  type="text"
                  value={properties.fields || ''}
                  onChange={(e) => handleChange('fields', e.target.value)}
                  placeholder="name, email, age"
                  className="input"
                />
                <p className="text-xs text-gray-500 mt-1">Comma-separated field names</p>
              </div>
            )}
            
            {(properties.type === 'filter' || selectedNode.data.definition?.type === 'filter') && (
              <>
                <div>
                  <label className="block text-sm font-medium mb-1">Field</label>
                  <input
                    type="text"
                    value={properties.field || ''}
                    onChange={(e) => handleChange('field', e.target.value)}
                    placeholder="age"
                    className="input"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium mb-1">Operator</label>
                  <select
                    value={properties.operator || 'eq'}
                    onChange={(e) => handleChange('operator', e.target.value)}
                    className="input"
                  >
                    <option value="eq">Equals (==)</option>
                    <option value="ne">Not Equals (!=)</option>
                    <option value="gt">Greater Than (&gt;)</option>
                    <option value="gte">Greater or Equal (&gt;=)</option>
                    <option value="lt">Less Than (&lt;)</option>
                    <option value="lte">Less or Equal (&lt;=)</option>
                    <option value="contains">Contains</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium mb-1">Value</label>
                  <input
                    type="text"
                    value={properties.value || ''}
                    onChange={(e) => handleChange('value', e.target.value)}
                    placeholder="18"
                    className="input"
                  />
                </div>
              </>
            )}
          </>
        )}
        
        {/* Connector Properties */}
        {nodeType === 'connector' && (
          <>
            <div>
              <label className="block text-sm font-medium mb-1">Connector Type</label>
              <input
                type="text"
                value={selectedNode.data.definition?.connector_type || ''}
                disabled
                className="input bg-gray-100 cursor-not-allowed"
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium mb-1">Connector Instance</label>
              <select
                value={properties.instance || ''}
                onChange={(e) => handleChange('instance', e.target.value)}
                className="input"
              >
                <option value="">Select instance...</option>
                {connectorInstances.map(inst => (
                  <option key={inst.id} value={inst.id}>
                    {inst.name} {inst.host && `(${inst.host})`}
                  </option>
                ))}
              </select>
              {connectorInstances.length === 0 && (
                <p className="text-xs text-orange-600 mt-1">
                  No instances found. Create one in Connectors page.
                </p>
              )}
            </div>
            
            <div>
              <label className="block text-sm font-medium mb-1">Operation</label>
              <select
                value={properties.operation || ''}
                onChange={(e) => handleChange('operation', e.target.value)}
                className="input"
              >
                <option value="">Select operation...</option>
                {selectedNode.data.definition?.operations?.map((op: any) => (
                  <option key={op.name} value={op.name}>
                    {op.name} - {op.description}
                  </option>
                ))}
              </select>
            </div>

            {/* Dynamic Parameters based on selected operation */}
            {selectedOperation && selectedOperation.parameters && (
              <div className="space-y-3 pt-2 border-t">
                <h4 className="font-medium text-sm">Operation Parameters</h4>
                {selectedOperation.parameters.map((param: any) => (
                  <div key={param.name}>
                    <label className="block text-sm font-medium mb-1">
                      {param.name}
                      {param.required && <span className="text-red-500 ml-1">*</span>}
                    </label>
                    
                    {/* For SQL query - use textarea */}
                    {param.name === 'sql' ? (
                      <>
                        <textarea
                          value={properties.params?.[param.name] || ''}
                          onChange={(e) => handleParamChange(param.name, e.target.value)}
                          placeholder={param.description || `Enter ${param.name}`}
                          className="input font-mono text-sm"
                          rows={4}
                          required={param.required}
                        />
                        <p className="text-xs text-gray-500 mt-1">
                          {param.description || 'SQL query to execute'}
                        </p>
                      </>
                    ) : param.param_type === 'object' ? (
                      /* For objects - use textarea with JSON */
                      <>
                        <textarea
                          value={
                            typeof properties.params?.[param.name] === 'object'
                              ? JSON.stringify(properties.params[param.name], null, 2)
                              : properties.params?.[param.name] || ''
                          }
                          onChange={(e) => {
                            try {
                              const parsed = JSON.parse(e.target.value)
                              handleParamChange(param.name, parsed)
                            } catch {
                              handleParamChange(param.name, e.target.value)
                            }
                          }}
                          placeholder={param.description || '{}'}
                          className="input font-mono text-sm"
                          rows={3}
                        />
                        <p className="text-xs text-gray-500 mt-1">
                          {param.description || 'JSON object'}
                        </p>
                      </>
                    ) : (
                      /* For strings and other types */
                      <>
                        <input
                          type="text"
                          value={properties.params?.[param.name] || ''}
                          onChange={(e) => handleParamChange(param.name, e.target.value)}
                          placeholder={param.description || `Enter ${param.name}`}
                          className="input"
                          required={param.required}
                        />
                        {param.description && (
                          <p className="text-xs text-gray-500 mt-1">{param.description}</p>
                        )}
                      </>
                    )}
                  </div>
                ))}
              </div>
            )}
          </>
        )}
        
        {/* Info */}
        <div className="bg-blue-50 border border-blue-200 rounded p-3 text-xs">
          <p className="font-medium text-blue-900">Node ID: {selectedNode.id}</p>
          <p className="text-blue-700 mt-1">Type: {nodeType}</p>
        </div>
      </div>
    </div>
  )
}
