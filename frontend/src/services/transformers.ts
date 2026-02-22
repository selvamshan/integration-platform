import { api } from './api'

export interface Transformer {
  id: string
  name: string
  description: string
  type: string
}

export const transformerService = {
  /**
   * Get all available transformer types
   * Uses GET /transformers/capabilities endpoint
   */
  async list(): Promise<{ transformers: Transformer[] }> {
    try {
      const response = await api.get('/transformers/capabilities')
      
      // Parse the response to extract transformer types
      const capabilities = response.data
      
      // Extract JSON transformers
      const jsonOps = capabilities.json?.operations || []
      
      const transformers: Transformer[] = jsonOps.map((op: any) => ({
        id: `transform-${op.type}`,
        name: op.type.charAt(0).toUpperCase() + op.type.slice(1),
        description: op.description,
        type: op.type,
      }))
      
      return { transformers }
    } catch (error) {
      console.error('Failed to load transformers:', error)
      
      // Fallback to hardcoded list
      return {
        transformers: [
          { id: 'transform-select', name: 'Select', description: 'Select specific fields', type: 'select' },
          { id: 'transform-map', name: 'Map', description: 'Transform array elements', type: 'map' },
          { id: 'transform-filter', name: 'Filter', description: 'Filter array by condition', type: 'filter' },
          { id: 'transform-rename', name: 'Rename', description: 'Rename fields', type: 'rename' },
          { id: 'transform-convert', name: 'Convert', description: 'Convert field types', type: 'convert' },
        ]
      }
    }
  },
}
