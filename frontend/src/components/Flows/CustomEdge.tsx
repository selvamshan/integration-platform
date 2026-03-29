import { memo } from 'react'
import { EdgeProps, getBezierPath, EdgeLabelRenderer, BaseEdge } from 'reactflow'

const CONDITION_COLORS: Record<string, string> = {
  always: '#94a3b8',
  on_success: '#22c55e',
  on_error: '#ef4444',
  custom: '#f59e0b',
}

const CONDITION_LABELS: Record<string, string> = {
  always: '',
  on_success: '✓ success',
  on_error: '✗ error',
}

export const CustomEdge = memo(({
  id,
  sourceX, sourceY, targetX, targetY,
  sourcePosition, targetPosition,
  data, selected, markerEnd,
}: EdgeProps) => {
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX, sourceY, sourcePosition,
    targetX, targetY, targetPosition,
  })

  const condition: string = data?.condition || 'always'
  const color = CONDITION_COLORS[condition] ?? '#94a3b8'

  const displayLabel =
    data?.label ||
    (condition === 'custom' ? (data?.expression || 'custom') : CONDITION_LABELS[condition]) ||
    ''

  return (
    <>
      <BaseEdge
        id={id}
        path={edgePath}
        markerEnd={markerEnd}
        style={{
          stroke: selected ? '#6366f1' : color,
          strokeWidth: selected ? 2.5 : 1.5,
        }}
      />

      {displayLabel && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: 'absolute',
              transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
              pointerEvents: 'all',
              color,
            }}
            className="nodrag nopan bg-white border border-gray-200 rounded px-1.5 py-0.5 text-xs shadow-sm"
          >
            {displayLabel}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  )
})

CustomEdge.displayName = 'CustomEdge'
