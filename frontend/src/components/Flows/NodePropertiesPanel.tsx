import { useState, useEffect } from 'react'
import { Node } from 'reactflow'
import { X } from 'lucide-react'
import { api } from '@/services/api'

// ─── SQL Param Builder ────────────────────────────────────────────────────────

type ParamSource = 'query_params' | 'path_params' | 'body' | 'headers' | 'custom'

const SOURCE_OPTIONS: { value: ParamSource; label: string; placeholder: string; prefix: string }[] = [
  { value: 'query_params', label: 'Query Param',  placeholder: 'e.g. name',    prefix: '{{trigger.query_params.' },
  { value: 'path_params',  label: 'Path Param',   placeholder: 'e.g. id',      prefix: '{{trigger.path_params.'  },
  { value: 'body',         label: 'Body Field',   placeholder: 'e.g. email',   prefix: '{{trigger.body.'         },
  { value: 'headers',      label: 'Header',       placeholder: 'e.g. x-token', prefix: '{{trigger.headers.'      },
  { value: 'custom',       label: 'Custom',       placeholder: '{{any.path}}', prefix: ''                        },
]

function parseParamValue(raw: string): { source: ParamSource; field: string } {
  for (const opt of SOURCE_OPTIONS) {
    if (opt.value !== 'custom' && raw.startsWith(opt.prefix) && raw.endsWith('}}')) {
      return { source: opt.value, field: raw.slice(opt.prefix.length, -2) }
    }
  }
  return { source: 'custom', field: raw }
}

function buildParamValue(source: ParamSource, field: string): string {
  if (source === 'custom') return field
  const opt = SOURCE_OPTIONS.find(o => o.value === source)!
  return field.trim() ? `${opt.prefix}${field.trim()}}}` : ''
}

interface SqlParamRowProps {
  index: number
  value: string
  onChange: (value: string) => void
}

function SqlParamRow({ index, value, onChange }: SqlParamRowProps) {
  const parsed = parseParamValue(value || '')
  const [source, setSource] = useState<ParamSource>(parsed.source)
  const [field,  setField]  = useState(parsed.field)

  // Re-sync when the parent resets (e.g. SQL change trims the array)
  useEffect(() => {
    const p = parseParamValue(value || '')
    setSource(p.source)
    setField(p.field)
  }, [value])

  const handleSource = (s: ParamSource) => {
    setSource(s)
    onChange(buildParamValue(s, field))
  }

  const handleField = (f: string) => {
    setField(f)
    onChange(buildParamValue(source, f))
  }

  const opt = SOURCE_OPTIONS.find(o => o.value === source)!

  return (
    <div className="flex items-center gap-2">
      <span className="text-xs font-mono text-purple-700 bg-purple-50 border border-purple-200 px-2 py-1.5 rounded shrink-0 w-8 text-center">
        ${index + 1}
      </span>
      <select
        value={source}
        onChange={e => handleSource(e.target.value as ParamSource)}
        className="input text-xs shrink-0"
        style={{ width: '118px' }}
      >
        {SOURCE_OPTIONS.map(o => (
          <option key={o.value} value={o.value}>{o.label}</option>
        ))}
      </select>
      <input
        type="text"
        value={field}
        onChange={e => handleField(e.target.value)}
        placeholder={opt.placeholder}
        className={`input text-sm flex-1 ${source === 'custom' ? 'font-mono' : ''}`}
      />
    </div>
  )
}

interface NodePropertiesPanelProps {
  selectedNode: Node | null
  onClose: () => void
  onUpdate: (nodeId: string, data: any) => void
  onRename?: (oldId: string, newName: string) => void
}

export function NodePropertiesPanel({
  selectedNode,
  onClose,
  onUpdate,
  onRename,
}: NodePropertiesPanelProps) {
  const [properties, setProperties] = useState<any>({})
  const [connectorInstances, setConnectorInstances] = useState<any[]>([])
  const [selectedOperation, setSelectedOperation] = useState<any>(null)
  const [nodeName, setNodeName] = useState('')

  useEffect(() => {
    if (selectedNode) {
      setNodeName(selectedNode.data.label || '')
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

  const handleSqlParamArrayChange = (index: number, value: string) => {
    const current: string[] = Array.isArray(properties.params?.params)
      ? [...properties.params.params]
      : []
    current[index] = value
    handleParamChange('params', current)
  }

  /** Count distinct $N placeholders in SQL, returns the max index found */
  const countSqlParams = (sql: string): number => {
    const matches = sql?.match(/\$(\d+)/g) || []
    if (matches.length === 0) return 0
    return Math.max(...matches.map(m => parseInt(m.slice(1), 10)))
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
            value={nodeName}
            onChange={(e) => setNodeName(e.target.value)}
            onBlur={() => {
              const trimmed = nodeName.trim()
              if (trimmed && trimmed !== selectedNode.id) {
                onRename?.(selectedNode.id, trimmed)
              }
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                const trimmed = nodeName.trim()
                if (trimmed && trimmed !== selectedNode.id) {
                  onRename?.(selectedNode.id, trimmed)
                }
                e.currentTarget.blur()
              }
            }}
            className="input"
          />
          <p className="text-xs text-gray-500 mt-1">Also used as the step ID in the flow</p>
        </div>
        
        {/* Trigger Properties */}
        {nodeType === 'trigger' && (() => {
          const triggerType =
            selectedNode.data.definition?.trigger_type ||
            (properties.cron ? 'schedule' : 'http')

          if (triggerType === 'schedule') {
            return (
              <>
                <div>
                  <label className="block text-sm font-medium mb-1">Cron Expression</label>
                  <input
                    type="text"
                    value={properties.cron || ''}
                    onChange={(e) => handleChange('cron', e.target.value)}
                    placeholder="0 * * * *"
                    className="input font-mono"
                  />
                  <p className="text-xs text-gray-500 mt-1">
                    min hr day month weekday — e.g. <code>0 3 * * *</code> = daily at 3 AM
                  </p>
                </div>
                <div>
                  <label className="block text-sm font-medium mb-1">Timezone</label>
                  <select
                    value={properties.timezone || 'UTC'}
                    onChange={(e) => handleChange('timezone', e.target.value)}
                    className="input"
                  >
                    <option value="UTC">UTC</option>
                    <option value="America/New_York">America/New_York (EST/EDT)</option>
                    <option value="America/Chicago">America/Chicago (CST/CDT)</option>
                    <option value="America/Denver">America/Denver (MST/MDT)</option>
                    <option value="America/Los_Angeles">America/Los_Angeles (PST/PDT)</option>
                    <option value="America/Sao_Paulo">America/Sao_Paulo (BRT)</option>
                    <option value="Europe/London">Europe/London (GMT/BST)</option>
                    <option value="Europe/Paris">Europe/Paris (CET/CEST)</option>
                    <option value="Europe/Berlin">Europe/Berlin (CET/CEST)</option>
                    <option value="Europe/Moscow">Europe/Moscow (MSK)</option>
                    <option value="Asia/Kolkata">Asia/Kolkata (IST)</option>
                    <option value="Asia/Shanghai">Asia/Shanghai (CST)</option>
                    <option value="Asia/Tokyo">Asia/Tokyo (JST)</option>
                    <option value="Asia/Singapore">Asia/Singapore (SGT)</option>
                    <option value="Asia/Dubai">Asia/Dubai (GST)</option>
                    <option value="Australia/Sydney">Australia/Sydney (AEST/AEDT)</option>
                    <option value="Pacific/Auckland">Pacific/Auckland (NZST/NZDT)</option>
                  </select>
                  <p className="text-xs text-gray-500 mt-1">Timezone for cron expression evaluation</p>
                </div>
                <div className="bg-blue-50 border border-blue-200 rounded p-3 text-xs text-blue-800 space-y-1">
                  <p className="font-medium">Common patterns</p>
                  <p><code>* * * * *</code> — every minute</p>
                  <p><code>0 * * * *</code> — every hour</p>
                  <p><code>0 0 * * *</code> — daily at midnight</p>
                  <p><code>0 9 * * 1-5</code> — weekdays at 9 AM</p>
                  <p><code>0 0 * * 0</code> — weekly on Sunday</p>
                </div>
              </>
            )
          }

          // HTTP trigger (default)
          return (
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
          )
        })()}
        
        {/* Log Properties */}
        {nodeType === 'log' && (
          <>
            <div>
              <label className="block text-sm font-medium mb-1">Step Name</label>
              <input
                type="text"
                value={properties.name || ''}
                onChange={(e) => handleChange('name', e.target.value)}
                placeholder="e.g. log-request"
                className="input"
              />
              <p className="text-xs text-gray-500 mt-1">Unique identifier for this log step</p>
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Message</label>
              <textarea
                value={properties.message || ''}
                onChange={(e) => handleChange('message', e.target.value)}
                placeholder="e.g. Rate limited endpoint called"
                className="input"
                rows={3}
              />
              <p className="text-xs text-gray-500 mt-1">Message to log during flow execution</p>
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
        
        {/* Rate Limit Properties */}
        {nodeType === 'rate_limit' && (
          <>
            <div>
              <label className="block text-sm font-medium mb-1">Max Requests</label>
              <input
                type="number"
                min={1}
                value={properties.max_requests ?? 10}
                onChange={(e) => handleChange('max_requests', Number(e.target.value))}
                className="input"
              />
              <p className="text-xs text-gray-500 mt-1">Maximum requests allowed in the window</p>
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Window (seconds)</label>
              <input
                type="number"
                min={1}
                value={properties.window_seconds ?? 60}
                onChange={(e) => handleChange('window_seconds', Number(e.target.value))}
                className="input"
              />
              <p className="text-xs text-gray-500 mt-1">Time window for the request count</p>
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Key Type</label>
              <select
                value={properties.key_type || 'per_ip'}
                onChange={(e) => handleChange('key_type', e.target.value)}
                className="input"
              >
                <option value="per_ip">Per IP</option>
                <option value="per_user">Per User</option>
                <option value="global">Global</option>
              </select>
              <p className="text-xs text-gray-500 mt-1">Scope for counting requests</p>
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Rejection Message</label>
              <input
                type="text"
                value={properties.message || ''}
                onChange={(e) => handleChange('message', e.target.value)}
                placeholder="Too many requests, please try again later."
                className="input"
              />
              <p className="text-xs text-gray-500 mt-1">Message returned when limit is exceeded</p>
            </div>
            <div className="bg-red-50 border border-red-200 rounded p-3 text-xs text-red-800">
              This node configures flow-level rate limiting and is not a step — it will appear as <code>rate_limit</code> at the top of the flow YAML.
            </div>
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
                    
                    {/* For SQL query - use textarea + auto param inputs */}
                    {param.name === 'sql' ? (
                      <>
                        <textarea
                          value={properties.params?.[param.name] || ''}
                          onChange={(e) => {
                            const newSql = e.target.value
                            const newCount = countSqlParams(newSql)
                            const currentParams = Array.isArray(properties.params?.params)
                              ? properties.params.params
                              : []
                            const trimmedParams = currentParams.slice(0, newCount)
                            const updatedParams = { ...properties.params, sql: newSql, params: trimmedParams }
                            handleChange('params', updatedParams)
                          }}
                          placeholder={param.description || `Enter ${param.name}`}
                          className="input font-mono text-sm"
                          rows={4}
                          required={param.required}
                        />
                        <p className="text-xs text-gray-500 mt-1">
                          {param.description || 'SQL query to execute'}
                        </p>
                        {/* Auto-generate source+field rows for each $1, $2, … placeholder */}
                        {countSqlParams(properties.params?.sql || '') > 0 && (
                          <div className="mt-3 space-y-2">
                            <div className="flex items-center justify-between">
                              <p className="text-xs font-medium text-gray-700">Query Parameters</p>
                              <span className="text-xs text-gray-400 bg-gray-100 px-2 py-0.5 rounded">
                                {countSqlParams(properties.params?.sql || '')} param{countSqlParams(properties.params?.sql || '') > 1 ? 's' : ''}
                              </span>
                            </div>
                            {Array.from({ length: countSqlParams(properties.params?.sql || '') }, (_, i) => (
                              <SqlParamRow
                                key={i}
                                index={i}
                                value={(Array.isArray(properties.params?.params) ? properties.params.params[i] : '') || ''}
                                onChange={(v) => handleSqlParamArrayChange(i, v)}
                              />
                            ))}
                          </div>
                        )}
                      </>
                    ) : param.name === 'params' ? (
                      /* Skip — rendered inline above the SQL textarea */
                      null
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
