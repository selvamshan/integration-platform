import { useEffect, useState } from 'react'
import { useForm } from 'react-hook-form'
import { Connector } from '@/types/connector'
import { connectorDefinitionService, ConnectorDefinition } from '@/services/connectorDefinitions'
import { connectorService, TestConnectionResult } from '@/services/connector'

interface ConnectorFormProps {
  onSubmit: (data: Connector) => void
  initialValues?: Connector
  isEdit?: boolean
}

export function ConnectorForm({ onSubmit, initialValues, isEdit = false }: ConnectorFormProps) {
  const [definitions, setDefinitions] = useState<ConnectorDefinition[]>([])
  const [loadingDefs, setLoadingDefs] = useState(true)
  const [extraAttrsText, setExtraAttrsText] = useState(
    initialValues?.extra_attributes ? JSON.stringify(initialValues.extra_attributes, null, 2) : ''
  )
  const [extraAttrsError, setExtraAttrsError] = useState('')
  const [testing, setTesting] = useState(false)
  const [testResult, setTestResult] = useState<TestConnectionResult | null>(null)

  useEffect(() => {
    connectorDefinitionService.list()
      .then(({ connectors }) => setDefinitions(connectors))
      .catch(() => setDefinitions([]))
      .finally(() => setLoadingDefs(false))
  }, [])

  const { register, handleSubmit, watch, getValues, setValue } = useForm<Connector>({
    defaultValues: initialValues ?? { connector_type: 'postgres' },
  })

  useEffect(() => {
    if (!loadingDefs && isEdit && initialValues?.connector_type) {
      setValue('connector_type', initialValues.connector_type)
    }
  }, [loadingDefs, isEdit, initialValues, setValue])

  const connectorType = watch('connector_type')
  const isDb = connectorType === 'postgres' || connectorType === 'mysql' || connectorType === 'mssql' || connectorType === 'oracle'

  const DEFAULT_PORTS: Record<string, number> = { postgres: 5432, mysql: 3306, mssql: 1433, oracle: 1521 }

  useEffect(() => {
    if (!isEdit && connectorType in DEFAULT_PORTS) {
      setValue('port', DEFAULT_PORTS[connectorType])
    }
  }, [connectorType])

  function handleExtraAttrsChange(value: string) {
    setExtraAttrsText(value)
    if (value.trim() === '') {
      setExtraAttrsError('')
      return
    }
    try {
      JSON.parse(value)
      setExtraAttrsError('')
    } catch {
      setExtraAttrsError('Invalid JSON')
    }
  }

  async function handleTestConnection() {
    const data = getValues()
    setTesting(true)
    setTestResult(null)
    try {
      const result = await connectorService.testConnection(data)
      setTestResult(result)
    } catch (err: any) {
      const msg = err?.response?.data?.message ?? err?.message ?? 'Test failed'
      setTestResult({ success: false, message: msg })
    } finally {
      setTesting(false)
    }
  }

  function handleFormSubmit(data: Connector) {
    if (extraAttrsText.trim()) {
      try {
        data.extra_attributes = JSON.parse(extraAttrsText)
      } catch {
        setExtraAttrsError('Invalid JSON — fix before submitting')
        return
      }
    }
    // DB host must be a plain hostname or IP, not a URL
    const isDbConnector = data.connector_type === 'postgres' || data.connector_type === 'mysql' || data.connector_type === 'mssql' || data.connector_type === 'oracle'
    if (isDbConnector && data.host && /^https?:\/\//i.test(data.host)) {
      alert('Host must be a hostname or IP address (e.g. localhost), not a URL')
      return
    }
    onSubmit(data)
  }

  return (
    <form onSubmit={handleSubmit(handleFormSubmit)} className="space-y-4">
      <div className="grid grid-cols-2 gap-4">
        <input
          {...register('id')}
          placeholder="ID (unique)"
          className="input"
          required
          readOnly={isEdit}
          disabled={isEdit}
        />
        <input {...register('name')} placeholder="Display name" className="input" required />
      </div>

      <select {...register('connector_type')} className="input" required disabled={loadingDefs || isEdit}>
        {loadingDefs ? (
          <option value="">Loading connectors…</option>
        ) : definitions.length === 0 ? (
          <option value="">No connectors available</option>
        ) : (
          definitions.map((def) => (
            <option key={def.id} value={def.connector_type}>
              {def.icon} {def.name}
            </option>
          ))
        )}
      </select>

      {isDb && (
        <>
          <div className="grid grid-cols-3 gap-4">
            <input
              {...register('host')}
              placeholder="Host"
              className="input col-span-2"
              required
            />
            <input
              {...register('port', { valueAsNumber: true })}
              placeholder="Port"
              type="number"
              className="input"
              required
            />
          </div>
          <input {...register('database_name')} placeholder="Database name" className="input" required />
          <div className="grid grid-cols-2 gap-4">
            <input {...register('username')} placeholder="Username" className="input" required />
            <input
              {...register('password')}
              placeholder={isEdit ? 'Password (leave blank to keep current)' : 'Password'}
              type="password"
              className="input"
              required={!isEdit}
            />
          </div>
        </>
      )}

      {connectorType === 'http' && (
        <input {...register('host')} placeholder="Base URL (e.g. https://api.example.com)" className="input" required />
      )}

      <div className="space-y-1">
        <label className="text-xs text-gray-500 font-medium">Extra Attributes (JSON)</label>
        <textarea
          value={extraAttrsText}
          onChange={(e) => handleExtraAttrsChange(e.target.value)}
          placeholder={'{\n  "key": "value"\n}'}
          className={`input font-mono text-sm w-full resize-y${extraAttrsError ? ' border-red-500' : ''}`}
          rows={4}
          spellCheck={false}
        />
        {extraAttrsError && (
          <p className="text-xs text-red-500">{extraAttrsError}</p>
        )}
      </div>

      <div className="flex gap-2">
        <button
          type="button"
          onClick={handleTestConnection}
          disabled={testing}
          className="btn btn-secondary flex-1"
        >
          {testing ? 'Testing…' : 'Test Connection'}
        </button>
        <button type="submit" className="btn btn-primary flex-1">
          {isEdit ? 'Save Changes' : 'Create Connector'}
        </button>
      </div>

      {testResult && (
        <p className={`text-sm font-medium ${testResult.success ? 'text-green-600' : 'text-red-600'}`}>
          {testResult.success ? '✓' : '✗'} {testResult.message}
        </p>
      )}
    </form>
  )
}
