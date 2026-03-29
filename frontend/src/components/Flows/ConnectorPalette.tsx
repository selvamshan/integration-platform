import { useState, useEffect } from 'react'
import { connectorDefinitionService } from '@/services/connectorDefinitions'
import { triggerDefinitionService } from '@/services/triggerDefinitions'
import { transformerService } from '@/services/transformers'
import { Plus, ChevronDown, ChevronRight } from 'lucide-react'

interface PaletteProps {
  onAddNode: (type: string, data: any) => void
}

export function ConnectorPalette({ onAddNode }: PaletteProps) {
  const [connectors, setConnectors] = useState<any[]>([])
  const [triggers, setTriggers] = useState<any[]>([])
  const [transformers, setTransformers] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  
  // Expandable sections
  const [expandedSections, setExpandedSections] = useState({
    triggers: true,
    actions: true,
    transforms: true,
    connectors: true,
  })

  useEffect(() => {
    loadAll()
  }, [])

  const loadAll = async () => {
    try {
      const [connectorsData, triggersData, transformersData] = await Promise.all([
        connectorDefinitionService.list(),
        triggerDefinitionService.list(),
        transformerService.list(),
      ])
      
      setConnectors(connectorsData.connectors || [])
      setTriggers(triggersData.triggers || [])
      setTransformers(transformersData.transformers || [])
    } catch (error) {
      console.error('Failed to load palette:', error)
    } finally {
      setLoading(false)
    }
  }

  const toggleSection = (section: keyof typeof expandedSections) => {
    setExpandedSections(prev => ({
      ...prev,
      [section]: !prev[section]
    }))
  }

  if (loading) {
    return (
      <div className="w-64 border-r bg-gray-50 p-4">
        <div className="text-center text-gray-500">Loading palette...</div>
      </div>
    )
  }

  return (
    <div className="w-64 border-r bg-gray-50 overflow-y-auto">
      <div className="p-4 border-b bg-white sticky top-0 z-10">
        <h3 className="font-bold text-lg">Components</h3>
        <p className="text-xs text-gray-600 mt-1">
          Click to add to canvas
        </p>
      </div>

      <div className="p-3 space-y-3">
        {/* Triggers Section */}
        <Section
          title="Triggers"
          count={triggers.length}
          expanded={expandedSections.triggers}
          onToggle={() => toggleSection('triggers')}
        >
          {triggers.map((trigger) => (
            <PaletteItem
              key={trigger.id}
              icon={trigger.icon}
              name={trigger.name}
              description={trigger.description}
              color="blue"
              onClick={() => onAddNode('trigger', trigger)}
            />
          ))}
        </Section>

        {/* Actions Section */}
        <Section
          title="Actions"
          count={2}
          expanded={expandedSections.actions}
          onToggle={() => toggleSection('actions')}
        >
          <PaletteItem
            icon="📋"
            name="Log Info"
            description="Log a message to the flow execution output"
            color="green"
            onClick={() => onAddNode('log', {
              id: 'log-info',
              name: 'Log Info',
              icon: '📋',
              type: 'log',
            })}
          />
          <PaletteItem
            icon="🚦"
            name="Rate Limit"
            description="Limit requests per IP, user, or globally with a configurable window"
            color="red"
            onClick={() => onAddNode('rate_limit', {
              id: 'rate-limit',
              name: 'Rate Limit',
              icon: '🚦',
              type: 'rate_limit',
            })}
          />
        </Section>

        {/* Transforms Section */}
        <Section
          title="Transforms"
          count={transformers.length}
          expanded={expandedSections.transforms}
          onToggle={() => toggleSection('transforms')}
        >
          {transformers.map((transformer) => (
            <PaletteItem
              key={transformer.id}
              icon="🔄"
              name={transformer.name}
              description={transformer.description}
              color="amber"
              onClick={() => onAddNode('transform', transformer)}
            />
          ))}
        </Section>

        {/* Connectors Section */}
        <Section
          title="Connectors"
          count={connectors.length}
          expanded={expandedSections.connectors}
          onToggle={() => toggleSection('connectors')}
        >
          {connectors.map((connector) => (
            <PaletteItem
              key={connector.id}
              icon={connector.icon}
              name={connector.name}
              description={connector.description}
              operations={connector.operations?.map((op: any) => op.name)}
              color="purple"
              onClick={() => onAddNode('connector', connector)}
            />
          ))}
        </Section>
      </div>
    </div>
  )
}

// Section component
function Section({
  title,
  count,
  expanded,
  onToggle,
  children,
}: {
  title: string
  count: number
  expanded: boolean
  onToggle: () => void
  children: React.ReactNode
}) {
  return (
    <div className="bg-white rounded-lg border">
      <button
        onClick={onToggle}
        className="w-full px-3 py-2 flex items-center justify-between hover:bg-gray-50 rounded-t-lg"
      >
        <div className="flex items-center gap-2">
          {expanded ? (
            <ChevronDown className="w-4 h-4 text-gray-500" />
          ) : (
            <ChevronRight className="w-4 h-4 text-gray-500" />
          )}
          <span className="font-semibold text-sm">{title}</span>
          <span className="text-xs text-gray-500 bg-gray-100 px-2 py-0.5 rounded-full">
            {count}
          </span>
        </div>
      </button>
      
      {expanded && (
        <div className="p-2 space-y-1 border-t">
          {children}
        </div>
      )}
    </div>
  )
}

// Palette item component
function PaletteItem({
  icon,
  name,
  description,
  operations,
  color,
  onClick,
}: {
  icon: string
  name: string
  description: string
  operations?: string[]
  color: 'blue' | 'amber' | 'purple' | 'green' | 'red'
  onClick: () => void
}) {
  const colorClasses = {
    blue: 'bg-blue-50 border-blue-200 hover:bg-blue-100 hover:border-blue-300',
    amber: 'bg-amber-50 border-amber-200 hover:bg-amber-100 hover:border-amber-300',
    purple: 'bg-purple-50 border-purple-200 hover:bg-purple-100 hover:border-purple-300',
    green: 'bg-green-50 border-green-200 hover:bg-green-100 hover:border-green-300',
    red: 'bg-red-50 border-red-200 hover:bg-red-100 hover:border-red-300',
  }

  return (
    <button
      onClick={onClick}
      className={`w-full p-2 rounded border-2 transition-all text-left ${colorClasses[color]}`}
    >
      <div className="flex items-start gap-2">
        <span className="text-xl flex-shrink-0">{icon}</span>
        <div className="flex-1 min-w-0">
          <div className="font-medium text-sm truncate">{name}</div>
          <div className="text-xs text-gray-600 line-clamp-2">{description}</div>
          {operations && operations.length > 0 && (
            <div className="text-xs text-gray-500 mt-1">
              <span className="font-medium">Ops:</span> {operations.join(', ')}
            </div>
          )}
        </div>
        <Plus className="w-4 h-4 text-gray-400 flex-shrink-0 mt-0.5" />
      </div>
    </button>
  )
}
