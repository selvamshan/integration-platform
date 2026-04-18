import { useState, useRef, useEffect } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { ChevronDown, Settings, Users, LogOut, Shield } from 'lucide-react'
import { useAuthStore } from '@/store/authStore'
import { useSetupStore } from '@/store/setupStore'

export function Navbar() {
  const { user, logout } = useAuthStore()
  const { reset, provider } = useSetupStore()
  const navigate = useNavigate()
  const [adminOpen, setAdminOpen] = useState(false)
  const menuRef = useRef<HTMLDivElement>(null)

  // Close dropdown on outside click
  useEffect(() => {
    function handle(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setAdminOpen(false)
      }
    }
    document.addEventListener('mousedown', handle)
    return () => document.removeEventListener('mousedown', handle)
  }, [])

  const isAdmin = user?.roles?.includes('admin' as never) ?? false

  const providerLabel = { keycloak: 'Keycloak', auth0: 'Auth0', okta: 'Okta' }[provider] ?? provider

  const handleReconfigure = () => {
    setAdminOpen(false)
    reset()
    logout()
  }

  const handleManageUsers = () => {
    setAdminOpen(false)
    navigate('/users')
  }

  return (
    <nav className="bg-white shadow">
      <div className="container mx-auto px-4">
        <div className="flex justify-between items-center h-16">
          {/* Left nav */}
          <div className="flex gap-6 items-center">
            <Link to="/dashboard" className="font-bold text-xl">Integration Platform</Link>
            <Link to="/flows"       className="text-sm hover:text-primary-600">Flows</Link>
            <Link to="/connectors"  className="text-sm hover:text-primary-600">Connectors</Link>
            <Link to="/audit-logs"  className="text-sm hover:text-primary-600">Audit Logs</Link>
          </div>

          {/* Right actions */}
          <div className="flex items-center gap-3">
            <span className="text-sm text-gray-500">{user?.preferred_username}</span>

            {/* Admin dropdown — shown only to admins */}
            {isAdmin && (
              <div className="relative" ref={menuRef}>
                <button
                  onClick={() => setAdminOpen((o) => !o)}
                  className="flex items-center gap-1 rounded-lg border border-gray-200 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50 transition-colors"
                >
                  <Shield size={14} className="text-sky-600" />
                  Admin
                  <ChevronDown size={13} className={`transition-transform ${adminOpen ? 'rotate-180' : ''}`} />
                </button>

                {adminOpen && (
                  <div className="absolute right-0 mt-1 w-56 rounded-xl border border-gray-100 bg-white shadow-lg z-50 py-1">
                    {/* Provider badge */}
                    <div className="px-4 py-2 border-b border-gray-100">
                      <p className="text-xs text-gray-400 uppercase tracking-wide">Identity Provider</p>
                      <p className="text-sm font-medium text-gray-700 mt-0.5">{providerLabel}</p>
                    </div>

                    <button
                      onClick={handleManageUsers}
                      className="flex w-full items-center gap-2.5 px-4 py-2.5 text-sm text-gray-700 hover:bg-gray-50"
                    >
                      <Users size={15} className="text-gray-400" />
                      User Management
                    </button>

                    <button
                      onClick={handleReconfigure}
                      className="flex w-full items-center gap-2.5 px-4 py-2.5 text-sm text-gray-700 hover:bg-gray-50"
                    >
                      <Settings size={15} className="text-gray-400" />
                      Reconfigure OIDC
                    </button>

                    <div className="border-t border-gray-100 mt-1">
                      <button
                        onClick={() => { setAdminOpen(false); logout() }}
                        className="flex w-full items-center gap-2.5 px-4 py-2.5 text-sm text-red-600 hover:bg-red-50"
                      >
                        <LogOut size={15} />
                        Logout
                      </button>
                    </div>
                  </div>
                )}
              </div>
            )}

            {/* Non-admin logout */}
            {!isAdmin && (
              <button onClick={logout} className="btn btn-secondary">Logout</button>
            )}
          </div>
        </div>
      </div>
    </nav>
  )
}
