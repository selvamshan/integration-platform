# Frontend Navigation

The React UI is route-based (`react-router-dom`). The navbar exposes routes
based on authentication state and role; admin-only pages are reachable through
the **Admin** dropdown rather than the main nav bar.

## Routes

| Route | Page | Access | Description |
|---|---|---|---|
| `/setup` | Setup | Public | First-run wizard to configure the OIDC provider (Keycloak / Auth0 / Okta) |
| `/login` | Login | Public (requires setup) | Sign in via the configured identity provider |
| `/projects` | Projects | Authenticated | Default landing page; create/manage tenant-isolated projects |
| `/dashboard` | Dashboard | Authenticated | Summary cards for flows, connectors, and recent executions |
| `/flows` | Flows | Authenticated | List, create, and delete integration flows |
| `/flows/:id` | Flow Editor | Authenticated | Drag-and-drop React Flow canvas for building a flow's steps/graph |
| `/flows/:id/runs` | Flow Runs | Authenticated | Execution history and live run status for a flow |
| `/connectors` | Connectors | Authenticated | Register and manage connector instances (HTTP, Postgres, MySQL, MSSQL, Oracle, S3) |
| `/audit-logs` | Audit Logs | Authenticated | Searchable audit trail of platform actions |
| `/users` | Users | Admin only | Manage platform users and role assignments |
| `/clients` | API Clients | Admin only | Manage API client credentials |

## Guards

- **Setup guard** — every route except `/setup` redirects to `/setup` until an
  identity provider has been configured (`useSetupStore`).
- **Protected route** — routes nested under the main `Layout` redirect to
  `/login` if there is no auth token (`ProtectedRoute`).
- **Admin dropdown** — `/users` and `/clients` are only linked from the navbar
  for users whose JWT roles include `admin`; the routes themselves are not
  otherwise restricted client-side, so authorization is still enforced by the
  Control Plane API.

## Layout

`Layout` renders a persistent `Navbar` plus a router `<Outlet />` for the
active page. The navbar's left side links to Projects, Connectors, and Audit
Logs; the right side shows the current user and, for admins, a dropdown with
User Management, API Clients, and Reconfigure OIDC.

Source: `frontend/src/App.tsx`, `frontend/src/components/Layout/Navbar.tsx`,
`frontend/src/components/Layout/Layout.tsx`, `frontend/src/components/Auth/ProtectedRoute.tsx`.
