import { ConnectorInstance } from '@/types/connector'

const TYPE_LABEL: Record<string, string> = {
  postgres: 'PostgreSQL',
  mysql: 'MySQL',
  mssql:  'MS SQL Server',
  oracle: 'Oracle',
  http:   'HTTP',
}

export function ConnectorList({ connectors, onEdit, onDelete }: {
  connectors: ConnectorInstance[]
  onEdit: (connector: ConnectorInstance) => void
  onDelete: (id: string) => void
}) {
  if (connectors.length === 0) {
    return (
      <div className="text-center py-16 text-gray-500">
        No connectors yet. Click <strong>Create Connector</strong> to add one.
      </div>
    )
  }

  return (
    <div className="grid gap-4">
      {connectors.map((c) => (
        <div key={c.id} className="card flex items-center justify-between">
          <div>
            <div className="flex items-center gap-2">
              <h3 className="font-bold">{c.name}</h3>
              <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${
                c.active ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-500'
              }`}>
                {c.active ? 'active' : 'inactive'}
              </span>
            </div>
            <p className="text-sm text-gray-500 mt-0.5">
              {TYPE_LABEL[c.connector_type] ?? c.connector_type}
              {c.host && (
                <span className="ml-2 text-gray-400">
                  · {c.host}{c.port ? `:${c.port}` : ''}{c.database_name ? `/${c.database_name}` : ''}
                </span>
              )}
            </p>
            <p className="text-xs text-gray-400 mt-0.5">ID: {c.id}</p>
          </div>
          <div className="flex gap-2">
            <button
              onClick={() => onEdit(c)}
              className="btn btn-secondary"
            >
              Edit
            </button>
            <button
              onClick={() => onDelete(c.id)}
              className="btn btn-danger"
            >
              Delete
            </button>
          </div>
        </div>
      ))}
    </div>
  )
}
