export interface RateLimit {
  max_requests: number
  window_seconds: number
  key_type: 'per_ip' | 'per_user' | 'global'
  message?: string
}

// ─── Graph model ─────────────────────────────────────────────────────────────

export type EdgeCondition = 'always' | 'on_success' | 'on_error' | 'expression'

export interface FlowNode {
  id: string
  step: FlowStep
  position_x?: number
  position_y?: number
}

export interface FlowEdge {
  id: string
  from: string
  to: string
  condition: EdgeCondition
  expression?: string
}

// ─── Flow definition ─────────────────────────────────────────────────────────

export interface Flow {
  id: string
  name: string
  trigger: Trigger
  /** Graph execution model (preferred) */
  nodes: FlowNode[]
  edges: FlowEdge[]
  /** Legacy linear steps — kept for backwards compatibility */
  steps?: FlowStep[]
  rate_limit?: RateLimit
  active?: boolean
}

export type Trigger =
  | { type: 'http'; path: string; method: string }
  | { type: 'schedule'; cron: string; timezone?: string }

export interface FlowStep {
  type: 'log' | 'call' | 'transform' | 'rate_limit'
  name: string
  message?: string
  connector?: string
  operation?: string
  params?: any
  spec?: any
}
