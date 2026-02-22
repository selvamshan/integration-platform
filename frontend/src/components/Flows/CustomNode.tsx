import { memo } from 'react'
import { Handle, Position, NodeProps } from 'reactflow'
import { X } from 'lucide-react'

export const CustomNode = memo(({ data, selected, id }: NodeProps) => {
  const handleDelete = (e: React.MouseEvent) => {
    e.stopPropagation()
    if (data.onDelete) {
      data.onDelete(id)
    }
  }

  return (
    <div
      className={`
        relative px-4 py-3 rounded-lg border-2 shadow-md min-w-[150px]
        ${selected ? 'ring-2 ring-primary-500 ring-offset-2' : ''}
      `}
      style={{
        background: data.bgColor || '#fff',
        borderColor: data.borderColor || '#ccc',
      }}
    >
      {/* Delete button */}
      <button
        onClick={handleDelete}
        className="absolute -top-2 -right-2 w-5 h-5 bg-red-500 text-white rounded-full flex items-center justify-center hover:bg-red-600 transition-colors shadow-md"
        title="Delete node"
      >
        <X className="w-3 h-3" />
      </button>

      {/* Node content */}
      <div className="flex items-center gap-2">
        {data.icon && <span className="text-xl">{data.icon}</span>}
        <div className="flex-1">
          <div className="font-medium text-sm">{data.label}</div>
          {data.subtitle && (
            <div className="text-xs text-gray-600">{data.subtitle}</div>
          )}
        </div>
      </div>

      {/* Handles */}
      <Handle
        type="target"
        position={Position.Top}
        className="w-3 h-3 !bg-gray-400"
      />
      <Handle
        type="source"
        position={Position.Bottom}
        className="w-3 h-3 !bg-gray-400"
      />
    </div>
  )
})

CustomNode.displayName = 'CustomNode'
