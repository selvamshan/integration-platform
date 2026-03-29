import { Edge } from 'reactflow'
import { X, Trash2 } from 'lucide-react'

interface EdgePropertiesPanelProps {
  selectedEdge: Edge
  onClose: () => void
  onUpdate: (edgeId: string, data: any) => void
  onDelete: (edgeId: string) => void
}

export function EdgePropertiesPanel({
  selectedEdge,
  onClose,
  onUpdate,
  onDelete,
}: EdgePropertiesPanelProps) {
  const data = selectedEdge.data || {}

  const handleChange = (field: string, value: string) => {
    onUpdate(selectedEdge.id, { ...data, [field]: value })
  }

  const condition: string = data.condition || 'always'

  return (
    <div className="w-80 border-l bg-white overflow-y-auto">
      {/* Header */}
      <div className="p-4 border-b flex items-center justify-between sticky top-0 bg-white z-10">
        <h3 className="font-bold">Edge / Join Logic</h3>
        <button onClick={onClose} className="hover:bg-gray-100 p-1 rounded">
          <X className="w-4 h-4" />
        </button>
      </div>

      <div className="p-4 space-y-4">
        {/* Condition */}
        <div>
          <label className="block text-sm font-medium mb-1">Condition</label>
          <select
            value={condition}
            onChange={(e) => handleChange('condition', e.target.value)}
            className="input"
          >
            <option value="always">Always (unconditional)</option>
            <option value="on_success">On Success</option>
            <option value="on_error">On Error</option>
            <option value="custom">Custom Expression</option>
          </select>
          <p className="text-xs text-gray-500 mt-1">
            Controls when this connection is followed during execution.
          </p>
        </div>

        {/* Custom expression */}
        {condition === 'custom' && (
          <div>
            <label className="block text-sm font-medium mb-1">Expression</label>
            <input
              type="text"
              value={data.expression || ''}
              onChange={(e) => handleChange('expression', e.target.value)}
              placeholder="{{ output.status == 200 }}"
              className="input font-mono text-sm"
            />
            <p className="text-xs text-gray-500 mt-1">
              Use {'{{ }}'} to reference the previous step's output.
            </p>
          </div>
        )}

        {/* Optional label override */}
        <div>
          <label className="block text-sm font-medium mb-1">
            Label <span className="text-gray-400 font-normal">(optional)</span>
          </label>
          <input
            type="text"
            value={data.label || ''}
            onChange={(e) => handleChange('label', e.target.value)}
            placeholder="e.g. notify user"
            className="input"
          />
          <p className="text-xs text-gray-500 mt-1">
            Overrides the default label shown on the edge.
          </p>
        </div>

        {/* Info */}
        <div className="bg-gray-50 border border-gray-200 rounded p-3 text-xs text-gray-600 space-y-0.5">
          <p><span className="font-medium">From:</span> {selectedEdge.source}</p>
          <p><span className="font-medium">To:</span> {selectedEdge.target}</p>
        </div>

        {/* Delete */}
        <button
          onClick={() => onDelete(selectedEdge.id)}
          className="btn w-full flex items-center justify-center gap-2 text-red-600 border border-red-200 hover:bg-red-50"
        >
          <Trash2 className="w-4 h-4" />
          Delete Connection
        </button>
      </div>
    </div>
  )
}
