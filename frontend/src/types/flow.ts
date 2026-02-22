export interface Flow {
  id: string
  name: string
  trigger: Trigger
  steps: FlowStep[]
  active?: boolean
}

export interface Trigger {
  type: 'http'
  path: string
  method: string
}

export interface FlowStep {
  type: 'log' | 'call' | 'transform'
  name: string
  message?: string
  connector?: string
  operation?: string
  params?: any
  spec?: any
}
