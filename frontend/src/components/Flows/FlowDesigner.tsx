import { useCallback, useState } from 'react'
import ReactFlow, {
  Node,
  Edge,
  Controls,
  Background,
  useNodesState,
  useEdgesState,
  addEdge,
  Connection,
  MiniMap,
  Panel,
  NodeTypes,
} from 'reactflow'
import 'reactflow/dist/style.css'
import { Save, Play, Trash2 } from 'lucide-react'
import { ConnectorPalette } from './ConnectorPalette'
import { NodePropertiesPanel } from './NodePropertiesPanel'
import { CustomNode } from './CustomNode'

// Register custom node types
const nodeTypes: NodeTypes = {
  custom: CustomNode,
}

const initialNodes: Node[] = []
const initialEdges: Edge[] = []

let nodeId = 0
const getNodeId = () => `node_${nodeId++}`

export function FlowDesigner({ flowId, onSave }: { 
  flowId?: string
  onSave?: (flow: any) => void 
}) {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes)
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges)
  const [selectedNode, setSelectedNode] = useState<Node | null>(null)

  const onConnect = useCallback(
    (connection: Connection) => {
      setEdges((eds) => addEdge(connection, eds))
    },
    [setEdges]
  )

  const handleDeleteNode = useCallback((nodeId: string) => {
    setNodes((nds) => nds.filter((n) => n.id !== nodeId))
    setEdges((eds) => eds.filter((e) => e.source !== nodeId && e.target !== nodeId))
    if (selectedNode?.id === nodeId) {
      setSelectedNode(null)
    }
  }, [selectedNode, setNodes, setEdges])

  const handleAddNode = (type: string, data: any) => {
    const colors = {
      trigger: { bg: '#dbeafe', border: '#3b82f6' },
      transform: { bg: '#fef3c7', border: '#f59e0b' },
      connector: { bg: '#ddd6fe', border: '#8b5cf6' },
    }

    const color = colors[type as keyof typeof colors] || { bg: '#e5e7eb', border: '#6b7280' }

    const newNode: Node = {
      id: getNodeId(),
      type: 'custom', // Use custom node type
      position: {
        x: Math.random() * 400 + 100,
        y: Math.random() * 300 + 100,
      },
      data: {
        label: data.name,
        icon: data.icon,
        type,
        definition: data,
        bgColor: color.bg,
        borderColor: color.border,
        properties: {},
        onDelete: handleDeleteNode, // Pass delete handler
      },
    }

    setNodes((nds) => [...nds, newNode])
  }

  const handleNodeClick = useCallback((_event: any, node: Node) => {
    setSelectedNode(node)
  }, [])

  const handleUpdateNode = useCallback((nodeId: string, data: any) => {
    setNodes((nds) =>
      nds.map((node) =>
        node.id === nodeId ? { ...node, data } : node
      )
    )
    // Update selected node reference
    setSelectedNode(prev => prev?.id === nodeId ? { ...prev, data } as Node : prev)
  }, [setNodes])

  const handleClosePanel = useCallback(() => {
    setSelectedNode(null)
  }, [])

  const handleSave = () => {
    const triggerNode = nodes.find(n => n.data.type === 'trigger')
    
    const flow = {
      id: flowId || 'new-flow',
      name: 'My Flow',
      trigger: {
        type: 'http',
        path: triggerNode?.data.properties?.path || '/test',
        method: triggerNode?.data.properties?.method || 'POST',
      },
      steps: nodes
        .filter(n => n.data.type !== 'trigger')
        .map((node) => {
          if (node.data.type === 'transform') {
            return {
              type: 'transform',
              name: node.id,
              spec: {
                type: node.data.properties?.type || 'select',
                ...node.data.properties,
              }
            }
          } else if (node.data.type === 'connector') {
            return {
              type: 'call',
              name: node.id,
              connector: node.data.properties?.instance || '',
              operation: node.data.properties?.operation || '',
              params: node.data.properties?.params || {},
            }
          }
          return {
            type: 'log',
            name: node.id,
            message: 'Log message',
          }
        }),
    }
    
    if (onSave) {
      onSave(flow)
    } else {
      console.log('Flow:', flow)
      alert('Flow saved! (Check console)')
    }
  }

  const handleTest = () => {
    alert('Test flow execution (not implemented yet)')
  }

  const clearFlow = () => {
    if (confirm('Clear all nodes and edges?')) {
      setNodes([])
      setEdges([])
      setSelectedNode(null)
      nodeId = 0
    }
  }

  return (
    <div className="h-[calc(100vh-200px)] border rounded-lg bg-white flex">
      {/* Left Palette */}
      <ConnectorPalette onAddNode={handleAddNode} />

      {/* Main Canvas Area */}
      <div className="flex-1 flex flex-col">
        {/* Top Toolbar */}
        <div className="h-14 border-b px-4 flex items-center justify-between bg-gray-50">
          <div className="flex gap-2">
            <button
              onClick={handleSave}
              className="btn btn-primary flex items-center gap-2"
            >
              <Save className="w-4 h-4" />
              Save Flow
            </button>
            <button
              onClick={handleTest}
              className="btn btn-secondary flex items-center gap-2"
            >
              <Play className="w-4 h-4" />
              Test
            </button>
            <button
              onClick={clearFlow}
              className="btn btn-secondary flex items-center gap-2 text-red-600 hover:bg-red-50"
            >
              <Trash2 className="w-4 h-4" />
              Clear
            </button>
          </div>
          
          <div className="text-sm text-gray-600">
            {nodes.length} nodes, {edges.length} connections
          </div>
        </div>

        {/* Canvas + Properties Panel */}
        <div className="flex-1 flex">
          {/* ReactFlow Canvas */}
          <div className="flex-1">
            <ReactFlow
              nodes={nodes}
              edges={edges}
              onNodesChange={onNodesChange}
              onEdgesChange={onEdgesChange}
              onConnect={onConnect}
              onNodeClick={handleNodeClick}
              nodeTypes={nodeTypes}
              fitView
              attributionPosition="bottom-right"
            >
              <Background />
              <Controls />
              <MiniMap
                nodeColor={(node) => {
                  switch (node.data.type) {
                    case 'trigger': return '#3b82f6'
                    case 'transform': return '#f59e0b'
                    case 'connector': return '#8b5cf6'
                    default: return '#6b7280'
                  }
                }}
                style={{ height: 100 }}
              />
              <Panel position="top-right" className="bg-white p-2 rounded shadow text-xs">
                💡 Click nodes to configure • Hover X to delete
              </Panel>
            </ReactFlow>
          </div>

          {/* Right Properties Panel */}
          {selectedNode && (
            <NodePropertiesPanel
              selectedNode={selectedNode}
              onClose={handleClosePanel}
              onUpdate={handleUpdateNode}
            />
          )}
        </div>
      </div>
    </div>
  )
}
