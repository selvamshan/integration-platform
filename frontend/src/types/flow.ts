export interface RateLimit {
  max_requests: number
  window_seconds: number
  key_type: 'per_ip' | 'per_user' | 'global'
  message?: string
}

export interface Flow {
  id: string
  name: string
  trigger: Trigger
  steps: FlowStep[]
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
  // Edge routing — set by the flow designer based on outgoing edge conditions
  next?: string
  on_success?: string
  on_error?: string
  condition?: string
}
