# Control Plane

The Control Plane (`crates/control-plane`, port 8081) is the management API for the platform.

## Responsibilities

- **Flow Management** — Create, read, update, delete flow definitions
- **Connector Management** — Register and manage connector instances
- **User Management** — Invite users, manage roles
- **RBAC Enforcement** — Validate Keycloak JWTs and check permissions per request
- **Audit Logging** — Write tamper-evident audit records for every mutating action
- **Project Namespacing** — Isolate resources per project/tenant

## REST API

### Flows

| Method | Path | Permission | Description |
|--------|------|-----------|-------------|
| `GET` | `/flows` | `flows:read` | List all flows |
| `POST` | `/flows` | `flows:write` | Create a flow |
| `GET` | `/flows/:id` | `flows:read` | Get a flow |
| `PUT` | `/flows/:id` | `flows:write` | Update a flow |
| `DELETE` | `/flows/:id` | `flows:delete` | Delete a flow |

### Connectors

| Method | Path | Permission | Description |
|--------|------|-----------|-------------|
| `GET` | `/connector-instances` | `connectors:read` | List connector instances |
| `POST` | `/connector-instances` | `connectors:write` | Register a connector |
| `PUT` | `/connector-instances/:id` | `connectors:write` | Update a connector |
| `DELETE` | `/connector-instances/:id` | `connectors:delete` | Remove a connector |

### Users (Admin only)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/users/invite` | Invite a new user |
| `GET` | `/users` | List users |
| `DELETE` | `/users/:id` | Remove a user |

## Middleware Stack

```
Request
  → CORS
  → Tracing
  → rbac_middleware       (validate JWT, extract User)
  → permission_middleware (check User.can(required_permission))
  → Handler
  → Audit log write
```

## Database Schema

Key tables:

- `flow_definitions` — flow JSON, version, project_id
- `connector_instances` — connector config, encrypted credentials
- `connector_definitions` — connector type metadata
- `users` — username, email, role, hashed password
- `audit_logs` — action, actor, resource, timestamp, hash chain
- `projects` — project namespacing
- `trigger_definitions` — HTTP / scheduler trigger configs

Migrations live in `crates/control-plane/migrations/`.
