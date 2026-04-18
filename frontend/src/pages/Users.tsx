import { useEffect, useState } from 'react'
import { UserPlus, Trash2, Loader2, Shield, Users as UsersIcon } from 'lucide-react'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { api } from '@/services/api'

interface User {
  id: string
  username: string
  email: string
  name?: string
  roles: string[]
}

const inviteSchema = z.object({
  email: z.string().email('Valid email required'),
  role:  z.enum(['admin', 'developer', 'viewer']),
})
type InviteForm = z.infer<typeof inviteSchema>

const ROLE_COLORS: Record<string, string> = {
  admin:     'bg-red-100 text-red-700',
  developer: 'bg-sky-100 text-sky-700',
  viewer:    'bg-gray-100 text-gray-600',
}

export function Users() {
  const [users, setUsers]       = useState<User[]>([])
  const [loading, setLoading]   = useState(true)
  const [inviting, setInviting] = useState(false)
  const [deleting, setDeleting] = useState<string | null>(null)
  const [error, setError]       = useState('')
  const [success, setSuccess]   = useState('')

  const { register, handleSubmit, reset, formState: { errors } } = useForm<InviteForm>({
    resolver: zodResolver(inviteSchema),
    defaultValues: { role: 'viewer' },
  })

  const load = async () => {
    setLoading(true)
    try {
      const res = await api.get('/users')
      setUsers(res.data.users ?? [])
    } catch {
      setError('Failed to load users')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [])

  const onInvite = async (data: InviteForm) => {
    setInviting(true); setError(''); setSuccess('')
    try {
      await api.post('/users/invite', data)
      setSuccess(`Invitation sent to ${data.email}`)
      reset()
      load()
    } catch (e: any) {
      setError(e.response?.data?.message ?? 'Invitation failed')
    } finally {
      setInviting(false) }
  }

  const onDelete = async (userId: string, username: string) => {
    if (!confirm(`Delete user ${username}?`)) return
    setDeleting(userId); setError(''); setSuccess('')
    try {
      await api.delete(`/users/${userId}`)
      setSuccess(`User ${username} deleted`)
      load()
    } catch {
      setError('Delete failed')
    } finally {
      setDeleting(null)
    }
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl space-y-8">
      {/* Header */}
      <div className="flex items-center gap-3">
        <UsersIcon size={24} className="text-sky-600" />
        <div>
          <h1 className="text-2xl font-bold text-gray-900">User Management</h1>
          <p className="text-sm text-gray-500">Invite users and manage their roles</p>
        </div>
      </div>

      {/* Alerts */}
      {error   && <div className="rounded-lg bg-red-50 border border-red-200 px-4 py-3 text-sm text-red-700">{error}</div>}
      {success && <div className="rounded-lg bg-green-50 border border-green-200 px-4 py-3 text-sm text-green-700">{success}</div>}

      {/* Invite form */}
      <div className="card p-6">
        <h2 className="text-base font-semibold text-gray-800 mb-4 flex items-center gap-2">
          <UserPlus size={18} className="text-sky-500" /> Invite User
        </h2>
        <form onSubmit={handleSubmit(onInvite)} className="flex gap-3 items-end flex-wrap">
          <div className="flex-1 min-w-48">
            <label className="block text-sm font-medium text-gray-700 mb-1">Email</label>
            <input {...register('email')} placeholder="user@company.com" className="input w-full" />
            {errors.email && <p className="mt-1 text-xs text-red-600">{errors.email.message}</p>}
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Role</label>
            <select {...register('role')} className="input">
              <option value="admin">Admin</option>
              <option value="developer">Developer</option>
              <option value="viewer">Viewer</option>
            </select>
            {errors.role && <p className="mt-1 text-xs text-red-600">{errors.role.message}</p>}
          </div>
          <button type="submit" disabled={inviting} className="btn btn-primary flex items-center gap-2">
            {inviting ? <Loader2 size={15} className="animate-spin" /> : <UserPlus size={15} />}
            Send Invite
          </button>
        </form>
      </div>

      {/* User list */}
      <div className="card overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-100 flex items-center justify-between">
          <h2 className="text-base font-semibold text-gray-800">Users ({users.length})</h2>
          <button onClick={load} className="text-xs text-sky-600 hover:underline">Refresh</button>
        </div>

        {loading ? (
          <div className="flex items-center justify-center py-12 text-gray-400">
            <Loader2 size={24} className="animate-spin mr-2" /> Loading…
          </div>
        ) : users.length === 0 ? (
          <div className="py-12 text-center text-gray-400 text-sm">No users found</div>
        ) : (
          <ul className="divide-y divide-gray-100">
            {users.map((u) => (
              <li key={u.id} className="flex items-center justify-between px-6 py-4 hover:bg-gray-50">
                <div className="flex items-center gap-3">
                  <div className="flex h-9 w-9 items-center justify-center rounded-full bg-sky-100">
                    <Shield size={16} className="text-sky-600" />
                  </div>
                  <div>
                    <p className="text-sm font-medium text-gray-900">{u.username}</p>
                    <p className="text-xs text-gray-500">{u.email}</p>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <div className="flex gap-1">
                    {u.roles.map((r) => (
                      <span key={r} className={`rounded-full px-2 py-0.5 text-xs font-medium ${ROLE_COLORS[r] ?? 'bg-gray-100 text-gray-600'}`}>{r}</span>
                    ))}
                  </div>
                  <button
                    onClick={() => onDelete(u.id, u.username)}
                    disabled={deleting === u.id}
                    className="text-gray-400 hover:text-red-500 transition-colors disabled:opacity-50"
                    title="Delete user"
                  >
                    {deleting === u.id ? <Loader2 size={16} className="animate-spin" /> : <Trash2 size={16} />}
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
