import { useEffect, useState } from 'react'
import { useForm } from 'react-hook-form'
import { Connector } from '@/types/connector'
import { connectorDefinitionService, ConnectorDefinition } from '@/services/connectorDefinitions'

export function ConnectorForm({ onSubmit }: { onSubmit: (data: Connector) => void }) {
  const [definitions, setDefinitions] = useState<ConnectorDefinition[]>([])
  const [loadingDefs, setLoadingDefs] = useState(true)

  useEffect(() => {
    connectorDefinitionService.list()
      .then(({ connectors }) => setDefinitions(connectors))
      .catch(() => setDefinitions([]))
      .finally(() => setLoadingDefs(false))
  }, [])

  const { register, handleSubmit, watch } = useForm<Connector>({
    defaultValues: { connector_type: 'postgres' },
  })

  const connectorType = watch('connector_type')
  const isDb = connectorType === 'postgres' || connectorType === 'mysql'

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
      <div className="grid grid-cols-2 gap-4">
        <input {...register('id')} placeholder="ID (unique)" className="input" required />
        <input {...register('name')} placeholder="Display name" className="input" required />
      </div>

      <select {...register('connector_type')} className="input" required disabled={loadingDefs}>
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
              defaultValue={connectorType === 'mysql' ? 3306 : 5432}
              required
            />
          </div>
          <input {...register('database_name')} placeholder="Database name" className="input" required />
          <div className="grid grid-cols-2 gap-4">
            <input {...register('username')} placeholder="Username" className="input" required />
            <input {...register('password')} placeholder="Password" type="password" className="input" required />
          </div>
        </>
      )}

      {connectorType === 'http' && (
        <input {...register('host')} placeholder="Base URL (e.g. https://api.example.com)" className="input" required />
      )}

      <button type="submit" className="btn btn-primary w-full">Create Connector</button>
    </form>
  )
}
