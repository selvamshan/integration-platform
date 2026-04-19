import { useCallback, useEffect, useRef, useState } from 'react'
import { flowService, TestFlowResult } from '@/services/flow'
import { connectorDefinitionService } from '@/services/connectorDefinitions'
import { api } from '@/services/api'
import type { Flow, FlowStep, EdgeCondition } from '@/types/flow'
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
  EdgeTypes,
} from 'reactflow'
import 'reactflow/dist/style.css'
import { Save, Play, Trash2, Eye, Copy, Check, X } from 'lucide-react'
import { ConnectorPalette } from './ConnectorPalette'
import { NodePropertiesPanel } from './NodePropertiesPanel'
import { EdgePropertiesPanel } from './EdgePropertiesPanel'
import { CustomNode } from './CustomNode'
import { CustomEdge } from './CustomEdge'

const nodeTypes: NodeTypes = { custom: CustomNode }
const edgeTypes: EdgeTypes = { custom: CustomEdge }

// ── Lightweight YAML serializer ───────────────────────────────────────────
function yamlValue(v: any): string {
  if (v === null || v === undefined) return 'null'
  if (typeof v === 'boolean' || typeof v === 'number') return String(v)
  if (typeof v === 'string') {
    if (v === '') return '""'
    if (/[:#\{\}\[\],&*?|\-<>=!%@`\n\r]/.test(v) || /^\s|\s$/.test(v)) return JSON.stringify(v)
    return v
  }
  return JSON.stringify(v)
}

function toYaml(value: any, indent = 0): string {
  const pad = '  '.repeat(indent)
  if (value === null || value === undefined) return `${pad}null`
  if (typeof value !== 'object') return `${pad}${yamlValue(value)}`

  if (Array.isArray(value)) {
    if (value.length === 0) return `${pad}[]`
    return value.map((item) => {
      if (item !== null && typeof item === 'object' && !Array.isArray(item)) {
        const entries = Object.entries(item).filter(([, v]) => v !== undefined)
        if (entries.length === 0) return `${pad}- {}`
        const lines: string[] = []
        entries.forEach(([k, v], i) => {
          const prefix = i === 0 ? `${pad}- ` : `${pad}  `
          if (Array.isArray(v)) {
            lines.push(`${prefix}${k}:`)
            lines.push(toYaml(v, indent + 2))
          } else if (v !== null && typeof v === 'object') {
            lines.push(`${prefix}${k}:`)
            lines.push(toYaml(v, indent + 2))
          } else {
            lines.push(`${prefix}${k}: ${yamlValue(v)}`)
          }
        })
        return lines.join('\n')
      }
      return `${pad}- ${yamlValue(item)}`
    }).join('\n')
  }

  // plain object
  const entries = Object.entries(value).filter(([, v]) => v !== undefined)
  if (entries.length === 0) return `${pad}{}`
  return entries.map(([k, v]) => {
    if (Array.isArray(v)) {
      if (v.length === 0) return `${pad}${k}: []`
      return `${pad}${k}:\n${toYaml(v, indent + 1)}`
    }
    if (v !== null && typeof v === 'object') {
      return `${pad}${k}:\n${toYaml(v, indent + 1)}`
    }
    return `${pad}${k}: ${yamlValue(v)}`
  }).join('\n')
}

const initialNodes: Node[] = []
const initialEdges: Edge[] = []

let nodeId = 0
const getNodeId = () => `node_${nodeId++}`

export function FlowDesigner({ flowId, onSave, initialFlow }: {
  flowId?: string
  onSave?: (flow: any) => void
  initialFlow?: Flow
}) {
  const generatedId = useRef(crypto.randomUUID())
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes)
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges)
  const [selectedNode, setSelectedNode] = useState<Node | null>(null)
  const [selectedEdge, setSelectedEdge] = useState<Edge | null>(null)
  const [showYaml, setShowYaml] = useState(false)
  const [yamlText, setYamlText] = useState('')
  const [copied, setCopied] = useState(false)
  const [flowName, setFlowName] = useState('My Flow')
  const [customFlowId, setCustomFlowId] = useState('')
  const [saving, setSaving] = useState(false)
  const [saveError, setSaveError] = useState<string | null>(null)
  const [saveSuccess, setSaveSuccess] = useState<string | null>(null)
  const [showTestInput, setShowTestInput] = useState(false)
  const [testing, setTesting] = useState(false)
  const [testResult, setTestResult] = useState<TestFlowResult | null>(null)
  const [testInput, setTestInput] = useState('')

  // HTTP tester state
  const [showHttpTester, setShowHttpTester] = useState(false)
  const [httpMethod, setHttpMethod] = useState('POST')
  const [httpPath, setHttpPath] = useState('/api/webhook')
  const [httpActiveTab, setHttpActiveTab] = useState<'params' | 'headers' | 'body'>('body')
  const [httpQueryParams, setHttpQueryParams] = useState<{ key: string; value: string }[]>([{ key: '', value: '' }])
  const [httpHeaders, setHttpHeaders] = useState<{ key: string; value: string }[]>([{ key: 'Content-Type', value: 'application/json' }])
  const [httpBody, setHttpBody] = useState('{\n  \n}')
  const [httpSending, setHttpSending] = useState(false)
  const [httpResponse, setHttpResponse] = useState<{ status: number; statusText: string; headers: Record<string, string>; body: string; durationMs: number } | null>(null)

  // ── Connections ──────────────────────────────────────────────────────────
  const onConnect = useCallback(
    (connection: Connection) => {
      setEdges((eds) =>
        addEdge(
          { ...connection, type: 'custom', data: { condition: 'always' } },
          eds,
        ),
      )
    },
    [setEdges],
  )

  // ── Node handlers ─────────────────────────────────────────────────────────
  const handleDeleteNode = useCallback(
    (nodeId: string) => {
      setNodes((nds) => nds.filter((n) => n.id !== nodeId))
      setEdges((eds) => eds.filter((e) => e.source !== nodeId && e.target !== nodeId))
      if (selectedNode?.id === nodeId) setSelectedNode(null)
    },
    [selectedNode, setNodes, setEdges],
  )

  const handleAddNode = (type: string, data: any) => {
    const colors = {
      trigger:    { bg: '#dbeafe', border: '#3b82f6' },
      transform:  { bg: '#fef3c7', border: '#f59e0b' },
      connector:  { bg: '#ddd6fe', border: '#8b5cf6' },
      log:        { bg: '#dcfce7', border: '#22c55e' },
      rate_limit: { bg: '#fee2e2', border: '#ef4444' },
    }
    const color = colors[type as keyof typeof colors] || { bg: '#e5e7eb', border: '#6b7280' }

    const newNode: Node = {
      id: getNodeId(),
      type: 'custom',
      position: { x: Math.random() * 400 + 100, y: Math.random() * 300 + 100 },
      data: {
        label: data.name,
        icon: data.icon,
        type,
        definition: data,
        bgColor: color.bg,
        borderColor: color.border,
        properties: {},
        onDelete: handleDeleteNode,
      },
    }
    setNodes((nds) => [...nds, newNode])
  }

  const handleNodeClick = useCallback((_event: any, node: Node) => {
    setSelectedNode(node)
    setSelectedEdge(null)
  }, [])

  const handleUpdateNode = useCallback(
    (nodeId: string, data: any) => {
      setNodes((nds) => nds.map((n) => (n.id === nodeId ? { ...n, data } : n)))
      setSelectedNode((prev) => (prev?.id === nodeId ? { ...prev, data } as Node : prev))
    },
    [setNodes],
  )

  const handleRenameNode = useCallback(
    (oldId: string, newName: string) => {
      if (!newName.trim() || newName === oldId) return
      setNodes((nds) =>
        nds.map((n) =>
          n.id === oldId
            ? { ...n, id: newName, data: { ...n.data, label: newName } }
            : n,
        ),
      )
      setEdges((eds) =>
        eds.map((e) => {
          const newSource = e.source === oldId ? newName : e.source
          const newTarget = e.target === oldId ? newName : e.target
          const newEdgeId =
            newSource !== e.source || newTarget !== e.target
              ? `${newSource}→${newTarget}`
              : e.id
          return { ...e, id: newEdgeId, source: newSource, target: newTarget }
        }),
      )
      setSelectedNode((prev) =>
        prev?.id === oldId
          ? ({ ...prev, id: newName, data: { ...prev.data, label: newName } } as Node)
          : prev,
      )
    },
    [setNodes, setEdges],
  )

  const handleCloseNodePanel = useCallback(() => setSelectedNode(null), [])

  // ── Edge handlers ─────────────────────────────────────────────────────────
  const handleEdgeClick = useCallback((_event: any, edge: Edge) => {
    setSelectedEdge(edge)
    setSelectedNode(null)
  }, [])

  const handleUpdateEdge = useCallback(
    (edgeId: string, data: any) => {
      setEdges((eds) => eds.map((e) => (e.id === edgeId ? { ...e, data } : e)))
      setSelectedEdge((prev) => (prev?.id === edgeId ? { ...prev, data } as Edge : prev))
    },
    [setEdges],
  )

  const handleDeleteEdge = useCallback(
    (edgeId: string) => {
      setEdges((eds) => eds.filter((e) => e.id !== edgeId))
      setSelectedEdge(null)
    },
    [setEdges],
  )

  const handleCloseEdgePanel = useCallback(() => setSelectedEdge(null), [])

  // ── Load initial flow (edit mode) ─────────────────────────────────────────
  useEffect(() => {
    if (!initialFlow) return

    const colors = {
      trigger:    { bg: '#dbeafe', border: '#3b82f6' },
      transform:  { bg: '#fef3c7', border: '#f59e0b' },
      connector:  { bg: '#ddd6fe', border: '#8b5cf6' },
      log:        { bg: '#dcfce7', border: '#22c55e' },
      rate_limit: { bg: '#fee2e2', border: '#ef4444' },
    }

    const doLoad = async () => {
      setFlowName(initialFlow.name)

      // Build lookup maps: connector_type → definition, instance_id → connector_type
      const connectorDefsMap = new Map<string, any>()
      const instanceTypeMap = new Map<string, string>()
      try {
        const [defsData, instancesRes] = await Promise.all([
          connectorDefinitionService.list(),
          api.get('/connector-instances'),
        ])
        for (const def of defsData.connectors ?? []) {
          connectorDefsMap.set(def.connector_type, def)
        }
        for (const inst of (instancesRes.data?.connectors ?? [])) {
          instanceTypeMap.set(inst.id, inst.connector_type)
        }
      } catch (err) {
        console.error('Failed to load connector definitions for flow edit:', err)
      }

      const newNodes: Node[] = []
      const newEdges: Edge[] = []

      // Trigger node
      const isScheduleTrigger = initialFlow.trigger.type === 'schedule'
      const triggerProperties = isScheduleTrigger
        ? { cron: (initialFlow.trigger as any).cron, timezone: (initialFlow.trigger as any).timezone || 'UTC' }
        : { path: (initialFlow.trigger as any).path, method: (initialFlow.trigger as any).method }
      newNodes.push({
        id: 'trigger_node',
        type: 'custom',
        position: { x: 300, y: 50 },
        data: {
          label: isScheduleTrigger ? 'Schedule Trigger' : 'HTTP Trigger',
          icon: isScheduleTrigger ? '🕐' : '⚡',
          type: 'trigger',
          definition: { trigger_type: initialFlow.trigger.type },
          bgColor: colors.trigger.bg,
          borderColor: colors.trigger.border,
          properties: triggerProperties,
          onDelete: handleDeleteNode,
        },
      })

      const stepIcons: Record<string, string> = { log: '📋', call: '🔌', transform: '⚙️' }

      const buildStepNode = (step: FlowStep, id: string, pos: { x: number; y: number }): Node => {
        const visualType = step.type === 'call' ? 'connector' : step.type
        const color = colors[visualType as keyof typeof colors] ?? { bg: '#e5e7eb', border: '#6b7280' }
        const properties =
          step.type === 'call'
            ? { instance: step.connector, operation: step.operation, params: step.params }
            : step.type === 'transform'
            ? step.spec
            : { name: step.name, message: step.message }
        let definition: any = {}
        if (step.type === 'call') {
          const connectorType = instanceTypeMap.get(step.connector ?? '')
          if (connectorType) definition = connectorDefsMap.get(connectorType) ?? {}
        }
        return {
          id,
          type: 'custom',
          position: pos,
          data: {
            label: id,
            icon: stepIcons[step.type] ?? '📦',
            type: visualType,
            bgColor: color.bg,
            borderColor: color.border,
            definition,
            properties,
            onDelete: handleDeleteNode,
          },
        }
      }

      if (initialFlow.nodes.length > 0) {
        // ── Graph flow: restore from nodes + edges ──────────────────────────
        for (const fn of initialFlow.nodes) {
          newNodes.push(buildStepNode(fn.step, fn.id, {
            x: fn.position_x ?? 300,
            y: fn.position_y ?? 200,
          }))
        }
        const validNodeIds = new Set(initialFlow.nodes.map((n) => n.id))
        const validEdges = initialFlow.edges.filter(
          (fe) => validNodeIds.has(fe.from) && validNodeIds.has(fe.to),
        )
        for (const fe of validEdges) {
          newEdges.push({
            id: fe.id,
            source: fe.from,
            target: fe.to,
            type: 'custom',
            data: {
              condition: fe.condition === 'expression' ? 'custom' : fe.condition,
              expression: fe.expression,
            },
          })
        }
        // Connect trigger → first root node (nodes with no incoming edges)
        const targetIds = new Set(validEdges.map((e) => e.to))
        const rootNodes = initialFlow.nodes.filter((n) => !targetIds.has(n.id))
        for (const root of rootNodes) {
          newEdges.push({
            id: `trigger_node→${root.id}`,
            source: 'trigger_node',
            target: root.id,
            type: 'custom',
            data: { condition: 'always' },
          })
        }
      } else {
        // ── Legacy linear flow: restore from steps ──────────────────────────
        const steps = initialFlow.steps ?? []
        steps.forEach((step: FlowStep, i: number) => {
          newNodes.push(buildStepNode(step, step.name, { x: 300, y: 200 + i * 150 }))
        })
        // Trigger → first step edge
        if (steps.length > 0) {
          newEdges.push({
            id: `trigger_node→${steps[0].name}`,
            source: 'trigger_node',
            target: steps[0].name,
            type: 'custom',
            data: { condition: 'always' },
          })
          // Chain steps sequentially
          for (let i = 0; i < steps.length - 1; i++) {
            newEdges.push({
              id: `${steps[i].name}→${steps[i + 1].name}`,
              source: steps[i].name,
              target: steps[i + 1].name,
              type: 'custom',
              data: { condition: 'always' },
            })
          }
        }
      }

      // Rate limit node
      if (initialFlow.rate_limit) {
        newNodes.push({
          id: 'rate_limit_node',
          type: 'custom',
          position: { x: 600, y: 50 },
          data: {
            label: 'Rate Limit',
            icon: '🛡️',
            type: 'rate_limit',
            bgColor: colors.rate_limit.bg,
            borderColor: colors.rate_limit.border,
            properties: { ...initialFlow.rate_limit },
            onDelete: handleDeleteNode,
          },
        })
      }

      // Advance nodeId counter past any existing node_N IDs to avoid collisions
      let maxId = 0
      for (const n of initialFlow.nodes.length > 0 ? initialFlow.nodes : (initialFlow.steps ?? [])) {
        const nameStr = 'id' in n ? (n as any).id : (n as FlowStep).name
        const m = nameStr.match(/^node_(\d+)$/)
        if (m) maxId = Math.max(maxId, parseInt(m[1]) + 1)
      }
      nodeId = maxId

      setNodes(newNodes)
      setEdges(newEdges)
    }

    doLoad()
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialFlow])

  // ── Build flow object (shared by Save and View YAML) ─────────────────────
  const buildFlow = useCallback(() => {
    const triggerNode = nodes.find((n) => n.data.type === 'trigger')
    const rateLimitNode = nodes.find((n) => n.data.type === 'rate_limit')

    const rateLimit = rateLimitNode
      ? {
          max_requests: Number(rateLimitNode.data.properties?.max_requests) || 10,
          window_seconds: Number(rateLimitNode.data.properties?.window_seconds) || 60,
          key_type: rateLimitNode.data.properties?.key_type || 'per_ip',
          ...(rateLimitNode.data.properties?.message
            ? { message: rateLimitNode.data.properties.message }
            : {}),
        }
      : undefined

    const triggerType =
      triggerNode?.data.definition?.trigger_type ||
      (triggerNode?.data.properties?.cron ? 'schedule' : 'http')

    const trigger =
      triggerType === 'schedule'
        ? {
            type: 'schedule' as const,
            cron: triggerNode?.data.properties?.cron || '0 * * * *',
            timezone: triggerNode?.data.properties?.timezone || 'UTC',
          }
        : {
            type: 'http' as const,
            path: triggerNode?.data.properties?.path || '/test',
            method: triggerNode?.data.properties?.method || 'POST',
          }

    // Build FlowNodes (exclude trigger + rate_limit visual nodes)
    const flowNodes = nodes
      .filter((n) => n.data.type !== 'trigger' && n.data.type !== 'rate_limit')
      .map((node) => {
        let step: FlowStep
        if (node.data.type === 'transform') {
          step = {
            type: 'transform',
            name: node.id,
            spec: { type: node.data.properties?.type || 'select', ...node.data.properties },
          }
        } else if (node.data.type === 'connector') {
          const rawParams = { ...(node.data.properties?.params || {}) }
          if (typeof rawParams.sql === 'string' && Array.isArray(rawParams.params)) {
            const matches = rawParams.sql.match(/\$(\d+)/g) || []
            const count = matches.length === 0 ? 0 : Math.max(...matches.map((m: string) => parseInt(m.slice(1), 10)))
            rawParams.params = rawParams.params.slice(0, count)
          }
          step = {
            type: 'call',
            name: node.id,
            connector: node.data.properties?.instance || '',
            operation: node.data.properties?.operation || '',
            params: rawParams,
          }
        } else {
          step = {
            type: 'log',
            name: node.data.properties?.name || node.id,
            message: node.data.properties?.message || 'Log message',
          }
        }
        return {
          id: node.id,
          step,
          position_x: node.position.x,
          position_y: node.position.y,
        }
      })

    // Build FlowEdges — skip edges connected to trigger/rate_limit nodes
    // (trigger connectivity is implicit; the graph executor starts from root nodes)
    const triggerIds = new Set(nodes.filter((n) => n.data.type === 'trigger').map((n) => n.id))
    const rateLimitIds = new Set(nodes.filter((n) => n.data.type === 'rate_limit').map((n) => n.id))
    const flowEdges = edges
      .filter((e) => !triggerIds.has(e.source) && !triggerIds.has(e.target))
      .filter((e) => !rateLimitIds.has(e.source) && !rateLimitIds.has(e.target))
      .map((edge) => ({
        id: edge.id,
        from: edge.source,
        to: edge.target,
        condition: (edge.data?.condition === 'custom'
          ? 'expression'
          : edge.data?.condition ?? 'always') as EdgeCondition,
        ...(edge.data?.expression ? { expression: edge.data.expression } : {}),
      }))

    return {
      id: flowId || customFlowId.trim() || generatedId.current,
      name: flowName,
      trigger,
      nodes: flowNodes,
      edges: flowEdges,
      ...(rateLimit ? { rate_limit: rateLimit } : {}),
    }
  }, [nodes, edges, flowId, flowName, customFlowId])

  // ── Save ──────────────────────────────────────────────────────────────────
  const handleSave = async () => {
    const flow = buildFlow()
    if (onSave) {
      onSave(flow)
      return
    }
    setSaving(true)
    setSaveError(null)
    setSaveSuccess(null)
    try {
      if (flowId) {
        await flowService.update(flowId, flow)
        setSaveSuccess('Flow updated successfully')
      } else {
        await flowService.create(flow)
        setSaveSuccess('Flow saved successfully')
      }
      setTimeout(() => setSaveSuccess(null), 3000)
    } catch (err: any) {
      setSaveError(err?.response?.data?.error ?? err?.message ?? 'Save failed')
    } finally {
      setSaving(false)
    }
  }

  // ── View YAML ─────────────────────────────────────────────────────────────
  const handleViewYaml = () => {
    setYamlText(toYaml(buildFlow()))
    setShowYaml(true)
  }

  const handleCopyYaml = () => {
    navigator.clipboard.writeText(yamlText).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    })
  }

  const isScheduleFlow = useCallback(() => {
    const triggerNode = nodes.find((n) => n.data.type === 'trigger')
    const triggerType =
      triggerNode?.data.definition?.trigger_type ||
      (triggerNode?.data.properties?.cron ? 'schedule' : 'http')
    return triggerType === 'schedule'
  }, [nodes])

  const getHttpTriggerProps = useCallback(() => {
    const triggerNode = nodes.find((n) => n.data.type === 'trigger')
    return {
      path: triggerNode?.data.properties?.path || '/api/webhook',
      method: triggerNode?.data.properties?.method || 'POST',
    }
  }, [nodes])

  const handleOpenTestDialog = () => {
    if (isScheduleFlow()) {
      // Schedule: run immediately without dialog
      handleTestSchedule()
    } else {
      // HTTP: show Postman-like tester
      const { path, method } = getHttpTriggerProps()
      setHttpPath(path)
      setHttpMethod(method)
      setHttpResponse(null)
      setShowHttpTester(true)
    }
  }

  const handleTestSchedule = async () => {
    const flow = buildFlow()
    const now = new Date().toISOString()
    const scheduleContext = {
      type: 'schedule',
      scheduled_time: now,
      execution_time: now,
      flow_id: flow.id,
      flow_name: flow.name,
    }
    setTesting(true)
    setTestResult(null)
    try {
      const result = await flowService.test(flow, scheduleContext)
      setTestResult(result)
    } catch (err: any) {
      setTestResult({
        success: false,
        result: null,
        error: err?.response?.data?.error ?? err?.message ?? 'Test failed',
        execution: { duration_ms: 0, steps_executed: 0, step_results: [] },
      })
    } finally {
      setTesting(false)
    }
  }

  const handleTest = async () => {
    setShowTestInput(false)
    const flow = buildFlow()
    setTesting(true)
    setTestResult(null)
    try {
      let parsed: any = {}
      if (testInput.trim()) {
        try { parsed = JSON.parse(testInput) } catch { /* leave empty */ }
      }
      const result = await flowService.test(flow, parsed)
      setTestResult(result)
    } catch (err: any) {
      setTestResult({
        success: false,
        result: null,
        error: err?.response?.data?.error ?? err?.message ?? 'Test failed',
        execution: { duration_ms: 0, steps_executed: 0, step_results: [] },
      })
    } finally {
      setTesting(false)
    }
  }

  const handleSendHttpRequest = async () => {
    const dataplaneUrl = import.meta.env.VITE_DATA_PLANE_URL || 'http://localhost:8080'
    const params = httpQueryParams.filter((p) => p.key.trim())
    const qs = params.length ? '?' + params.map((p) => `${encodeURIComponent(p.key)}=${encodeURIComponent(p.value)}`).join('&') : ''
    const url = `${dataplaneUrl}${httpPath}${qs}`
    const headers: Record<string, string> = {}
    httpHeaders.filter((h) => h.key.trim()).forEach((h) => { headers[h.key] = h.value })

    setHttpSending(true)
    const t0 = Date.now()
    try {
      const res = await fetch(url, {
        method: httpMethod,
        headers,
        body: ['GET', 'HEAD'].includes(httpMethod) ? undefined : httpBody,
      })
      const durationMs = Date.now() - t0
      const resHeaders: Record<string, string> = {}
      res.headers.forEach((v, k) => { resHeaders[k] = v })
      const text = await res.text()
      let body = text
      try { body = JSON.stringify(JSON.parse(text), null, 2) } catch { /* not JSON */ }
      setHttpResponse({ status: res.status, statusText: res.statusText, headers: resHeaders, body, durationMs })
    } catch (err: any) {
      setHttpResponse({ status: 0, statusText: 'Network Error', headers: {}, body: err?.message ?? 'Request failed', durationMs: Date.now() - t0 })
    } finally {
      setHttpSending(false)
    }
  }

  const clearFlow = () => {
    if (confirm('Clear all nodes and edges?')) {
      setNodes([])
      setEdges([])
      setSelectedNode(null)
      setSelectedEdge(null)
      nodeId = 0
    }
  }

  return (
    <>
    {/* Save / Update toast */}
    {saveSuccess && (
      <div className="fixed bottom-6 left-1/2 -translate-x-1/2 z-50 flex items-center gap-2 bg-green-600 text-white text-sm font-medium px-5 py-2.5 rounded-full shadow-lg animate-fade-in">
        <svg className="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
        </svg>
        {saveSuccess}
      </div>
    )}
    <div className="h-[calc(100vh-160px)] border rounded-lg bg-white flex">
      {/* Left Palette */}
      <ConnectorPalette onAddNode={handleAddNode} />

      {/* Main Canvas Area */}
      <div className="flex-1 flex flex-col">
        {/* Top Toolbar */}
        <div className="border-b bg-gray-50 px-4 py-2 space-y-2">
          {/* Row 1 — Flow ID + Name inputs */}
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-1.5">
              <label className="text-xs font-medium text-gray-600 whitespace-nowrap">Flow ID</label>
              {flowId ? (
                <input
                  value={flowId}
                  disabled
                  className="input text-sm h-8 w-44 bg-gray-100 cursor-not-allowed"
                />
              ) : (
                <input
                  value={customFlowId}
                  onChange={(e) => setCustomFlowId(e.target.value)}
                  placeholder={generatedId.current}
                  className="input text-sm h-8 w-44 font-mono"
                />
              )}
            </div>
            <div className="flex items-center gap-1.5 flex-1">
              <label className="text-xs font-medium text-gray-600 whitespace-nowrap">Flow Name</label>
              <input
                value={flowName}
                onChange={(e) => setFlowName(e.target.value)}
                placeholder="My Flow"
                className="input text-sm h-8 flex-1 max-w-xs"
              />
            </div>
            <div className="text-xs text-gray-500 ml-auto">
              {nodes.length} nodes · {edges.length} connections
            </div>
          </div>

          {/* Row 2 — Action buttons */}
          <div className="flex items-center gap-2">
            <button onClick={handleSave} disabled={saving} className="btn btn-primary flex items-center gap-1.5 text-sm h-8">
              <Save className="w-3.5 h-3.5" />
              {saving ? (flowId ? 'Updating…' : 'Saving…') : (flowId ? 'Update Flow' : 'Save Flow')}
            </button>
            <button onClick={handleViewYaml} className="btn btn-secondary flex items-center gap-1.5 text-sm h-8">
              <Eye className="w-3.5 h-3.5" />
              View YAML
            </button>
            <button onClick={handleOpenTestDialog} disabled={testing} className="btn btn-secondary flex items-center gap-1.5 text-sm h-8">
              <Play className="w-3.5 h-3.5" />
              {testing ? 'Testing…' : 'Test'}
            </button>
            <button
              onClick={clearFlow}
              className="btn btn-secondary flex items-center gap-1.5 text-sm h-8 text-red-600 hover:bg-red-50"
            >
              <Trash2 className="w-3.5 h-3.5" />
              Clear
            </button>
            {saveError && (
              <span className="text-xs text-red-600 ml-2">{saveError}</span>
            )}
            {saveSuccess && (
              <span className="text-xs text-green-700 ml-2">{saveSuccess}</span>
            )}
          </div>
        </div>

        {/* Canvas + Properties Panel */}
        <div className="flex-1 flex">
          <div className="flex-1">
            <ReactFlow
              nodes={nodes}
              edges={edges}
              onNodesChange={onNodesChange}
              onEdgesChange={onEdgesChange}
              onConnect={onConnect}
              onNodeClick={handleNodeClick}
              onEdgeClick={handleEdgeClick}
              nodeTypes={nodeTypes}
              edgeTypes={edgeTypes}
              fitView
              attributionPosition="bottom-right"
            >
              <Background />
              <Controls />
              <MiniMap
                nodeColor={(node) => {
                  switch (node.data.type) {
                    case 'trigger':    return '#3b82f6'
                    case 'transform':  return '#f59e0b'
                    case 'connector':  return '#8b5cf6'
                    case 'log':        return '#22c55e'
                    case 'rate_limit': return '#ef4444'
                    default:           return '#6b7280'
                  }
                }}
                style={{ height: 100 }}
              />
              <Panel position="top-right" className="bg-white p-2 rounded shadow text-xs">
                💡 Click nodes/edges to configure • X to delete node
              </Panel>
            </ReactFlow>
          </div>

          {/* Right panel — node or edge properties */}
          {selectedNode && (
            <NodePropertiesPanel
              selectedNode={selectedNode}
              onClose={handleCloseNodePanel}
              onUpdate={handleUpdateNode}
              onRename={handleRenameNode}
            />
          )}
          {selectedEdge && (
            <EdgePropertiesPanel
              selectedEdge={selectedEdge}
              onClose={handleCloseEdgePanel}
              onUpdate={handleUpdateEdge}
              onDelete={handleDeleteEdge}
            />
          )}
        </div>
      </div>
    </div>

    {/* Test input dialog */}
    {showTestInput && (
      <div
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        onClick={() => setShowTestInput(false)}
      >
        <div
          className="bg-white rounded-xl shadow-2xl w-[480px] flex flex-col"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="flex items-center justify-between px-5 py-4 border-b">
            <h2 className="font-bold text-base">{isScheduleFlow() ? 'Run Scheduled Flow' : 'Test Flow'}</h2>
            <button onClick={() => setShowTestInput(false)} className="hover:bg-gray-100 p-1.5 rounded">
              <X className="w-4 h-4" />
            </button>
          </div>
          <div className="p-5 space-y-3">
            <label className="text-sm font-medium text-gray-700">
              {isScheduleFlow()
                ? <>Trigger Context <span className="text-gray-400 font-normal">(auto-filled from schedule trigger)</span></>
                : <>Test Input <span className="text-gray-400 font-normal">(JSON, optional)</span></>}
            </label>
            <textarea
              value={testInput}
              onChange={(e) => setTestInput(e.target.value)}
              placeholder={'{\n  "key": "value"\n}'}
              rows={6}
              className="w-full border rounded-lg p-3 text-sm font-mono resize-y focus:outline-none focus:ring-2 focus:ring-blue-300"
            />
          </div>
          <div className="flex justify-end gap-2 px-5 pb-5">
            <button onClick={() => setShowTestInput(false)} className="btn btn-secondary text-sm">Cancel</button>
            <button onClick={handleTest} className="btn btn-primary flex items-center gap-1.5 text-sm">
              <Play className="w-3.5 h-3.5" />
              Run Test
            </button>
          </div>
        </div>
      </div>
    )}

    {/* HTTP Tester (Postman-like) */}
    {showHttpTester && (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => setShowHttpTester(false)}>
        <div className="bg-white rounded-xl shadow-2xl w-[760px] max-h-[90vh] flex flex-col" onClick={(e) => e.stopPropagation()}>
          {/* Header */}
          <div className="flex items-center justify-between px-5 py-3 border-b">
            <h2 className="font-bold text-base">Test HTTP Trigger</h2>
            <button onClick={() => setShowHttpTester(false)} className="hover:bg-gray-100 p-1.5 rounded"><X className="w-4 h-4" /></button>
          </div>

          {/* URL bar */}
          <div className="flex items-center gap-2 px-4 py-3 border-b bg-gray-50">
            <select
              value={httpMethod}
              onChange={(e) => setHttpMethod(e.target.value)}
              className="border rounded px-2 py-1.5 text-sm font-medium bg-white w-24 focus:outline-none focus:ring-2 focus:ring-blue-300"
            >
              {['GET', 'POST', 'PUT', 'PATCH', 'DELETE'].map((m) => (
                <option key={m}>{m}</option>
              ))}
            </select>
            <div className="flex-1 flex items-center border rounded bg-white overflow-hidden focus-within:ring-2 focus-within:ring-blue-300">
              <span className="text-xs text-gray-400 pl-2 pr-1 whitespace-nowrap">{import.meta.env.VITE_DATA_PLANE_URL || 'http://localhost:8080'}</span>
              <input
                value={httpPath}
                onChange={(e) => setHttpPath(e.target.value)}
                className="flex-1 py-1.5 pr-2 text-sm font-mono focus:outline-none"
                placeholder="/api/webhook"
              />
            </div>
            <button
              onClick={handleSendHttpRequest}
              disabled={httpSending}
              className="btn btn-primary flex items-center gap-1.5 text-sm px-4 py-1.5 whitespace-nowrap"
            >
              <Play className="w-3.5 h-3.5" />
              {httpSending ? 'Sending…' : 'Send'}
            </button>
          </div>

          {/* Tabs */}
          <div className="flex border-b text-sm">
            {(['params', 'headers', 'body'] as const).map((tab) => (
              <button
                key={tab}
                onClick={() => setHttpActiveTab(tab)}
                className={`px-4 py-2 capitalize font-medium border-b-2 transition-colors ${httpActiveTab === tab ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-800'}`}
              >
                {tab}
                {tab === 'params' && httpQueryParams.filter((p) => p.key.trim()).length > 0 && (
                  <span className="ml-1.5 text-xs bg-blue-100 text-blue-600 rounded-full px-1.5">{httpQueryParams.filter((p) => p.key.trim()).length}</span>
                )}
                {tab === 'headers' && httpHeaders.filter((h) => h.key.trim()).length > 0 && (
                  <span className="ml-1.5 text-xs bg-blue-100 text-blue-600 rounded-full px-1.5">{httpHeaders.filter((h) => h.key.trim()).length}</span>
                )}
              </button>
            ))}
          </div>

          {/* Tab content */}
          <div className="flex-1 overflow-auto min-h-0">
            {/* Params tab */}
            {httpActiveTab === 'params' && (
              <div className="p-4 space-y-2">
                {httpQueryParams.map((param, i) => (
                  <div key={i} className="flex gap-2">
                    <input
                      value={param.key}
                      onChange={(e) => setHttpQueryParams((ps) => ps.map((p, j) => j === i ? { ...p, key: e.target.value } : p))}
                      placeholder="Key"
                      className="flex-1 border rounded px-2 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-200"
                    />
                    <input
                      value={param.value}
                      onChange={(e) => setHttpQueryParams((ps) => ps.map((p, j) => j === i ? { ...p, value: e.target.value } : p))}
                      placeholder="Value"
                      className="flex-1 border rounded px-2 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-200"
                    />
                    <button onClick={() => setHttpQueryParams((ps) => ps.filter((_, j) => j !== i))} className="text-gray-400 hover:text-red-500 px-1">
                      <X className="w-4 h-4" />
                    </button>
                  </div>
                ))}
                <button onClick={() => setHttpQueryParams((ps) => [...ps, { key: '', value: '' }])} className="text-sm text-blue-600 hover:underline">+ Add param</button>
              </div>
            )}

            {/* Headers tab */}
            {httpActiveTab === 'headers' && (
              <div className="p-4 space-y-2">
                {httpHeaders.map((header, i) => (
                  <div key={i} className="flex gap-2">
                    <input
                      value={header.key}
                      onChange={(e) => setHttpHeaders((hs) => hs.map((h, j) => j === i ? { ...h, key: e.target.value } : h))}
                      placeholder="Key"
                      className="flex-1 border rounded px-2 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-200"
                    />
                    <input
                      value={header.value}
                      onChange={(e) => setHttpHeaders((hs) => hs.map((h, j) => j === i ? { ...h, value: e.target.value } : h))}
                      placeholder="Value"
                      className="flex-1 border rounded px-2 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-200"
                    />
                    <button onClick={() => setHttpHeaders((hs) => hs.filter((_, j) => j !== i))} className="text-gray-400 hover:text-red-500 px-1">
                      <X className="w-4 h-4" />
                    </button>
                  </div>
                ))}
                <button onClick={() => setHttpHeaders((hs) => [...hs, { key: '', value: '' }])} className="text-sm text-blue-600 hover:underline">+ Add header</button>
              </div>
            )}

            {/* Body tab */}
            {httpActiveTab === 'body' && (
              <div className="p-4 h-full">
                {['GET', 'HEAD'].includes(httpMethod) ? (
                  <p className="text-sm text-gray-400 italic">GET requests do not have a body.</p>
                ) : (
                  <textarea
                    value={httpBody}
                    onChange={(e) => setHttpBody(e.target.value)}
                    rows={8}
                    className="w-full border rounded-lg p-3 text-sm font-mono resize-y focus:outline-none focus:ring-2 focus:ring-blue-300"
                    placeholder={'{\n  "key": "value"\n}'}
                  />
                )}
              </div>
            )}
          </div>

          {/* Response panel */}
          {httpResponse !== null && (
            <div className="border-t bg-gray-50">
              <div className="flex items-center gap-3 px-4 py-2 border-b">
                <span className="text-sm font-medium">Response</span>
                <span className={`text-xs font-bold px-2 py-0.5 rounded ${httpResponse.status >= 200 && httpResponse.status < 300 ? 'bg-green-100 text-green-700' : 'bg-red-100 text-red-700'}`}>
                  {httpResponse.status} {httpResponse.statusText}
                </span>
                <span className="text-xs text-gray-500 ml-auto">{httpResponse.durationMs}ms</span>
              </div>
              <pre className="p-4 text-xs font-mono overflow-auto max-h-48 whitespace-pre-wrap">{httpResponse.body}</pre>
            </div>
          )}
        </div>
      </div>
    )}

    {/* Test result modal */}
    {testResult !== null && (
      <div
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        onClick={() => setTestResult(null)}
      >
        <div
          className="bg-white rounded-xl shadow-2xl w-[640px] max-h-[80vh] flex flex-col"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="flex items-center justify-between px-5 py-4 border-b">
            <h2 className="font-bold text-base flex items-center gap-2">
              {testResult.success ? '✅' : '❌'} Test Result
              <span className="text-xs font-normal text-gray-500">
                {testResult.execution.duration_ms}ms · {testResult.execution.steps_executed} steps
              </span>
            </h2>
            <button onClick={() => setTestResult(null)} className="hover:bg-gray-100 p-1.5 rounded">
              <X className="w-4 h-4" />
            </button>
          </div>
          <div className="flex-1 overflow-auto p-5 space-y-3 text-sm">
            {testResult.error && (
              <div className="text-red-600 bg-red-50 rounded p-3">{testResult.error}</div>
            )}
            {testResult.execution.step_results.map((s) => (
              <div key={s.name} className={`rounded border p-3 ${s.success ? 'border-green-200 bg-green-50' : 'border-red-200 bg-red-50'}`}>
                <div className="flex items-center justify-between mb-1">
                  <span className="font-medium">{s.name}</span>
                  <span className="text-xs text-gray-500">{s.step_type} · {s.duration_ms}ms</span>
                </div>
                {s.error && <div className="text-red-600 text-xs">{s.error}</div>}
                {s.output !== undefined && (
                  <pre className="text-xs mt-1 text-gray-700 overflow-auto max-h-24">{JSON.stringify(s.output, null, 2)}</pre>
                )}
              </div>
            ))}
            {testResult.execution.output !== undefined && (
              <div className="border border-gray-200 rounded p-3 bg-gray-50">
                <div className="text-xs font-medium text-gray-600 mb-1">Final Output</div>
                <pre className="text-xs overflow-auto max-h-32">{JSON.stringify(testResult.execution.output, null, 2)}</pre>
              </div>
            )}
          </div>
        </div>
      </div>
    )}

    {/* YAML modal */}
    {showYaml && (
      <div
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        onClick={() => setShowYaml(false)}
      >
        <div
          className="bg-white rounded-xl shadow-2xl w-[640px] max-h-[80vh] flex flex-col"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Modal header */}
          <div className="flex items-center justify-between px-5 py-4 border-b">
            <h2 className="font-bold text-base">Flow YAML</h2>
            <div className="flex items-center gap-2">
              <button
                onClick={handleCopyYaml}
                className="btn btn-secondary flex items-center gap-1.5 text-sm"
              >
                {copied ? <Check className="w-4 h-4 text-green-600" /> : <Copy className="w-4 h-4" />}
                {copied ? 'Copied!' : 'Copy'}
              </button>
              <button
                onClick={() => setShowYaml(false)}
                className="hover:bg-gray-100 p-1.5 rounded"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
          </div>

          {/* YAML content */}
          <pre className="flex-1 overflow-auto p-5 text-sm font-mono bg-gray-50 rounded-b-xl whitespace-pre text-gray-800">
            {yamlText}
          </pre>
        </div>
      </div>
    )}
    </>
  )
}
