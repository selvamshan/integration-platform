import { Link } from 'react-router-dom'
import { useAuthStore } from '@/store/authStore'

export function Navbar() {
  const { user, logout } = useAuthStore()
  
  return (
    <nav className="bg-white shadow">
      <div className="container mx-auto px-4">
        <div className="flex justify-between items-center h-16">
          <div className="flex gap-6">
            <Link to="/dashboard" className="font-bold text-xl">Integration Platform</Link>
            <Link to="/flows" className="hover:text-primary-600">Flows</Link>
            <Link to="/connectors" className="hover:text-primary-600">Connectors</Link>
          </div>
          <div className="flex items-center gap-4">
            <span>{user?.preferred_username}</span>
            <button onClick={logout} className="btn btn-secondary">Logout</button>
          </div>
        </div>
      </div>
    </nav>
  )
}
