import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { ChevronDown, ChevronRight, Folder, Pencil, Trash2, Plus, X } from 'lucide-react'
import { projectService } from '@/services/project'
import { Project } from '@/types/project'
import { Flow } from '@/types/flow'

interface ProjectFlows {
  loading: boolean
  flows: Flow[]
}

export function Projects() {
  const [projects, setProjects] = useState<Project[]>([])
  const [expanded, setExpanded] = useState<Record<string, boolean>>({})
  const [flowsMap, setFlowsMap] = useState<Record<string, ProjectFlows>>({})
  const [showForm, setShowForm] = useState(false)
  const [formName, setFormName] = useState('')
  const [formDesc, setFormDesc] = useState('')
  const [creating, setCreating] = useState(false)
  const [deletingId, setDeletingId] = useState<string | null>(null)
  const navigate = useNavigate()

  useEffect(() => {
    projectService.list().then((d) => setProjects(d.projects))
  }, [])

  const toggleProject = async (id: string) => {
    const isOpen = !expanded[id]
    setExpanded((prev) => ({ ...prev, [id]: isOpen }))

    if (isOpen && !flowsMap[id]) {
      setFlowsMap((prev) => ({ ...prev, [id]: { loading: true, flows: [] } }))
      try {
        const data = await projectService.listFlows(id)
        setFlowsMap((prev) => ({ ...prev, [id]: { loading: false, flows: data.flows } }))
      } catch {
        setFlowsMap((prev) => ({ ...prev, [id]: { loading: false, flows: [] } }))
      }
    }
  }

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!formName.trim()) return
    setCreating(true)
    try {
      const project = await projectService.create({
        name: formName.trim(),
        description: formDesc.trim() || undefined,
      })
      setProjects((prev) => [project, ...prev])
      setFormName('')
      setFormDesc('')
      setShowForm(false)
    } finally {
      setCreating(false)
    }
  }

  const handleDelete = async (id: string, name: string) => {
    if (!confirm(`Delete project "${name}"? Associated flows will not be deleted.`)) return
    setDeletingId(id)
    try {
      await projectService.delete(id)
      setProjects((prev) => prev.filter((p) => p.id !== id))
    } finally {
      setDeletingId(null)
    }
  }

  const refreshProjectFlows = async (projectId: string) => {
    setFlowsMap((prev) => ({ ...prev, [projectId]: { loading: true, flows: [] } }))
    const data = await projectService.listFlows(projectId)
    setFlowsMap((prev) => ({ ...prev, [projectId]: { loading: false, flows: data.flows } }))
  }

  return (
    <div>
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-3xl font-bold">Projects</h1>
        <button
          onClick={() => setShowForm((s) => !s)}
          className="btn btn-primary flex items-center gap-2"
        >
          <Plus className="w-4 h-4" />
          New Project
        </button>
      </div>

      {/* Create project form */}
      {showForm && (
        <div className="card mb-6">
          <div className="flex justify-between items-center mb-4">
            <h2 className="font-semibold text-lg">Create Project</h2>
            <button onClick={() => setShowForm(false)} className="text-gray-400 hover:text-gray-600">
              <X className="w-5 h-5" />
            </button>
          </div>
          <form onSubmit={handleCreate} className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Name</label>
              <input
                type="text"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
                placeholder="My Project"
                required
                className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Description (optional)</label>
              <input
                type="text"
                value={formDesc}
                onChange={(e) => setFormDesc(e.target.value)}
                placeholder="What this project does"
                className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
              />
            </div>
            <div className="flex gap-3">
              <button
                type="submit"
                disabled={creating || !formName.trim()}
                className="btn btn-primary disabled:opacity-50"
              >
                {creating ? 'Creating…' : 'Create Project'}
              </button>
              <button type="button" onClick={() => setShowForm(false)} className="btn btn-secondary">
                Cancel
              </button>
            </div>
          </form>
        </div>
      )}

      {/* Project list */}
      <div className="space-y-3">
        {projects.map((project) => {
          const isOpen = expanded[project.id] ?? false
          const projectFlows = flowsMap[project.id]

          return (
            <div key={project.id} className="card p-0 overflow-hidden">
              {/* Project header row */}
              <div
                className="flex items-center justify-between px-5 py-4 cursor-pointer hover:bg-gray-50 transition-colors"
                onClick={() => toggleProject(project.id)}
              >
                <div className="flex items-center gap-3 min-w-0">
                  {isOpen
                    ? <ChevronDown className="w-4 h-4 text-gray-400 flex-shrink-0" />
                    : <ChevronRight className="w-4 h-4 text-gray-400 flex-shrink-0" />
                  }
                  <Folder className="w-5 h-5 text-sky-500 flex-shrink-0" />
                  <div className="min-w-0">
                    <span className="font-semibold text-gray-900">{project.name}</span>
                    {project.description && (
                      <span className="ml-2 text-sm text-gray-500">{project.description}</span>
                    )}
                  </div>
                  <span className="ml-2 text-xs text-gray-400 flex-shrink-0">
                    {project.flow_count ?? 0} {project.flow_count === 1 ? 'flow' : 'flows'}
                  </span>
                </div>

                <div
                  className="flex items-center gap-2 flex-shrink-0"
                  onClick={(e) => e.stopPropagation()}
                >
                  <button
                    onClick={() => handleDelete(project.id, project.name)}
                    disabled={deletingId === project.id}
                    className="btn btn-secondary flex items-center gap-1.5 text-sm text-red-600 hover:bg-red-50 disabled:opacity-50"
                  >
                    <Trash2 className="w-4 h-4" />
                    {deletingId === project.id ? 'Deleting…' : 'Delete'}
                  </button>
                </div>
              </div>

              {/* Expanded: flows */}
              {isOpen && (
                <div className="border-t border-gray-100 bg-gray-50">
                  {projectFlows?.loading ? (
                    <p className="text-sm text-gray-400 px-6 py-4">Loading flows…</p>
                  ) : projectFlows?.flows.length === 0 ? (
                    <div className="px-6 py-4 flex items-center justify-between">
                      <p className="text-sm text-gray-500">No flows in this project yet.</p>
                      <button
                        onClick={() => navigate('/flows/new')}
                        className="btn btn-secondary text-sm flex items-center gap-1.5"
                      >
                        <Plus className="w-3.5 h-3.5" />
                        Create Flow
                      </button>
                    </div>
                  ) : (
                    <div className="divide-y divide-gray-100">
                      {projectFlows?.flows.map((flow) => (
                        <div key={flow.id} className="flex items-center justify-between px-6 py-3">
                          <div>
                            <p className="font-medium text-sm text-gray-900">{flow.name}</p>
                            <p className="text-xs text-gray-500 font-mono mt-0.5">
                              {flow.trigger.type === 'http'
                                ? `${flow.trigger.method} ${flow.trigger.path}`
                                : `schedule: ${flow.trigger.cron}`}
                            </p>
                          </div>
                          <div className="flex items-center gap-2">
                            <button
                              onClick={() => navigate(`/flows/${flow.id}`)}
                              className="btn btn-secondary flex items-center gap-1.5 text-sm"
                            >
                              <Pencil className="w-3.5 h-3.5" />
                              Edit
                            </button>
                          </div>
                        </div>
                      ))}
                      <div className="px-6 py-3 flex justify-end">
                        <button
                          onClick={() => navigate('/flows/new')}
                          className="btn btn-secondary text-sm flex items-center gap-1.5"
                        >
                          <Plus className="w-3.5 h-3.5" />
                          Add Flow
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          )
        })}

        {projects.length === 0 && (
          <p className="text-gray-500 text-center py-16">
            No projects yet. Create your first project to organise your flows.
          </p>
        )}
      </div>
    </div>
  )
}
