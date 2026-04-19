import { useEffect, useState } from 'react'
import { KeyRound, Plus, Trash2, Loader2, ToggleLeft, ToggleRight, Copy, Check, Eye } from 'lucide-react'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { api } from '@/services/api'

interface Client {
  client_id: string
  name: string
  active: boolean
  created_at: string
  expires_at: string | null
}

interface CreatedSecret {
  client_id: string
  client_secret: string
  name: string
  expires_at: string | null
  warning: string
}

const createSchema = z.object({
  name:            z.string().min(1, 'Name is required'),
  expires_in_days: z.string().optional(),
})
type CreateForm = z.infer<typeof createSchema>

export function Clients() {
  const [clients, setClients]       = useState<Client[]>([])
  const [loading, setLoading]       = useState(true)
  const [creating, setCreating]     = useState(false)
  const [deleting, setDeleting]     = useState<string | null>(null)
  const [toggling, setToggling]     = useState<string | null>(null)
  const [error, setError]           = useState('')
  const [revealed, setRevealed]     = useState<CreatedSecret | null>(null)
  const [copied, setCopied]         = useState<'id' | 'secret' | null>(null)

  const { register, handleSubmit, reset, formState: { errors } } = useForm<CreateForm>({
    resolver: zodResolver(createSchema),
  })

  const load = async () => {
    setLoading(true)
    try {
      const res = await api.get('/auth/clients')
      setClients(res.data.clients ?? [])
    } catch {
      setError('Failed to load clients')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [])

  const onCreate = async (data: CreateForm) => {
    setCreating(true); setError(''); setRevealed(null)
    try {
      const body: Record<string, unknown> = { name: data.name }
      if (data.expires_in_days) body.expires_in_days = parseInt(data.expires_in_days, 10)
      const res = await api.post('/auth/clients', body)
      setRevealed(res.data)
      reset()
      load()
    } catch (e: any) {
      setError(e.response?.data?.message ?? 'Create failed')
    } finally {
      setCreating(false)
    }
  }

  const onDelete = async (clientId: string, name: string) => {
    if (!confirm(`Delete client "${name}"?`)) return
    setDeleting(clientId); setError('')
    try {
      await api.delete(`/auth/clients/${clientId}`)
      if (revealed?.client_id === clientId) setRevealed(null)
      load()
    } catch {
      setError('Delete failed')
    } finally {
      setDeleting(null)
    }
  }

  const onToggle = async (client: Client) => {
    setToggling(client.client_id); setError('')
    try {
      await api.patch(`/auth/clients/${client.client_id}`, { active: !client.active })
      load()
    } catch {
      setError('Toggle failed')
    } finally {
      setToggling(null)
    }
  }

  const copy = (text: string, field: 'id' | 'secret') => {
    navigator.clipboard.writeText(text)
    setCopied(field)
    setTimeout(() => setCopied(null), 2000)
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl space-y-8">
      <div className="flex items-center gap-3">
        <KeyRound size={24} className="text-sky-600" />
        <div>
          <h1 className="text-2xl font-bold text-gray-900">API Clients</h1>
          <p className="text-sm text-gray-500">Manage machine credentials for the control-plane API</p>
        </div>
      </div>

      {error && (
        <div className="rounded-lg bg-red-50 border border-red-200 px-4 py-3 text-sm text-red-700">{error}</div>
      )}

      {/* Secret reveal panel — shown once after creation */}
      {revealed && (
        <div className="rounded-xl border border-amber-300 bg-amber-50 p-5 space-y-3">
          <div className="flex items-center gap-2 text-amber-800 font-semibold">
            <Eye size={16} /> Client created — copy credentials now
          </div>
          <p className="text-xs text-amber-700">{revealed.warning}</p>

          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="text-xs font-medium text-gray-600 w-28">Client ID</span>
              <code className="flex-1 rounded bg-white border border-amber-200 px-3 py-1.5 text-xs font-mono text-gray-800 truncate">
                {revealed.client_id}
              </code>
              <button
                onClick={() => copy(revealed.client_id, 'id')}
                className="text-amber-600 hover:text-amber-800 transition-colors"
                title="Copy"
              >
                {copied === 'id' ? <Check size={15} /> : <Copy size={15} />}
              </button>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-xs font-medium text-gray-600 w-28">Client Secret</span>
              <code className="flex-1 rounded bg-white border border-amber-200 px-3 py-1.5 text-xs font-mono text-gray-800 truncate">
                {revealed.client_secret}
              </code>
              <button
                onClick={() => copy(revealed.client_secret, 'secret')}
                className="text-amber-600 hover:text-amber-800 transition-colors"
                title="Copy"
              >
                {copied === 'secret' ? <Check size={15} /> : <Copy size={15} />}
              </button>
            </div>
          </div>

          <p className="text-xs text-amber-600">
            Use these with <code className="bg-white px-1 rounded border border-amber-200">POST /auth/token</code> to obtain a bearer token.
          </p>

          <button
            onClick={() => setRevealed(null)}
            className="text-xs text-amber-700 hover:underline"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Create form */}
      <div className="card p-6">
        <h2 className="text-base font-semibold text-gray-800 mb-4 flex items-center gap-2">
          <Plus size={16} className="text-sky-500" /> New Client
        </h2>
        <form onSubmit={handleSubmit(onCreate)} className="flex gap-3 items-end flex-wrap">
          <div className="flex-1 min-w-48">
            <label className="block text-sm font-medium text-gray-700 mb-1">Name</label>
            <input {...register('name')} placeholder="e.g. my-service" className="input w-full" />
            {errors.name && <p className="mt-1 text-xs text-red-600">{errors.name.message}</p>}
          </div>
          <div className="w-36">
            <label className="block text-sm font-medium text-gray-700 mb-1">Expires (days)</label>
            <input
              {...register('expires_in_days')}
              type="number"
              min="1"
              placeholder="never"
              className="input w-full"
            />
          </div>
          <button type="submit" disabled={creating} className="btn btn-primary flex items-center gap-2">
            {creating ? <Loader2 size={15} className="animate-spin" /> : <Plus size={15} />}
            Create
          </button>
        </form>
      </div>

      {/* Client list */}
      <div className="card overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-100 flex items-center justify-between">
          <h2 className="text-base font-semibold text-gray-800">Clients ({clients.length})</h2>
          <button onClick={load} className="text-xs text-sky-600 hover:underline">Refresh</button>
        </div>

        {loading ? (
          <div className="flex items-center justify-center py-12 text-gray-400">
            <Loader2 size={24} className="animate-spin mr-2" /> Loading…
          </div>
        ) : clients.length === 0 ? (
          <div className="py-12 text-center text-gray-400 text-sm">No clients yet</div>
        ) : (
          <ul className="divide-y divide-gray-100">
            {clients.map((c) => (
              <li key={c.client_id} className="flex items-center justify-between px-6 py-4 hover:bg-gray-50">
                <div className="flex items-center gap-3">
                  <div className="flex h-9 w-9 items-center justify-center rounded-full bg-sky-100">
                    <KeyRound size={15} className="text-sky-600" />
                  </div>
                  <div>
                    <p className="text-sm font-medium text-gray-900">{c.name}</p>
                    <p className="text-xs text-gray-400 font-mono">{c.client_id}</p>
                    {c.expires_at && (
                      <p className="text-xs text-gray-400">
                        Expires {new Date(c.expires_at).toLocaleDateString()}
                      </p>
                    )}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <span className={`rounded-full px-2 py-0.5 text-xs font-medium ${c.active ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-500'}`}>
                    {c.active ? 'active' : 'inactive'}
                  </span>
                  <button
                    onClick={() => onToggle(c)}
                    disabled={toggling === c.client_id}
                    className="text-gray-400 hover:text-sky-500 transition-colors disabled:opacity-50"
                    title={c.active ? 'Deactivate' : 'Activate'}
                  >
                    {toggling === c.client_id
                      ? <Loader2 size={18} className="animate-spin" />
                      : c.active ? <ToggleRight size={18} className="text-sky-500" /> : <ToggleLeft size={18} />}
                  </button>
                  <button
                    onClick={() => onDelete(c.client_id, c.name)}
                    disabled={deleting === c.client_id}
                    className="text-gray-400 hover:text-red-500 transition-colors disabled:opacity-50"
                    title="Delete"
                  >
                    {deleting === c.client_id ? <Loader2 size={16} className="animate-spin" /> : <Trash2 size={16} />}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}
