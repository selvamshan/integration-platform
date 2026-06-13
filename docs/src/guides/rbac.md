# RBAC & Keycloak SSO

The Control Plane is protected by Keycloak JWT validation and role-based access control.

## Roles

| Role | Description |
|------|-------------|
| `admin` | Full access including user management |
| `developer` | Create and manage flows, connectors, APIs |
| `viewer` | Read-only access |

## Role Permissions

| Permission | admin | developer | viewer |
|-----------|-------|-----------|--------|
| Flows: read | ✅ | ✅ | ✅ |
| Flows: write/delete | ✅ | ✅ | ✗ |
| Connectors: read | ✅ | ✅ | ✅ |
| Connectors: write/delete | ✅ | ✅ | ✗ |
| Users: invite/delete | ✅ | ✗ | ✗ |
| Users: list | ✅ | ✗ | ✗ |
| Metrics: read | ✅ | ✅ | ✅ |

## Authentication Flow

```
User → Keycloak (login) → JWT token
JWT token → Control Plane (Authorization: Bearer <token>)
Control Plane → validate JWT → extract roles → check permission → allow/403
```

## Keycloak Setup

### 1. Create a Realm

In Keycloak Admin UI (`http://localhost:8082`):

1. Create realm: `integration-platform`
2. Create client: `control-plane` (Client Protocol: `openid-connect`, Access Type: `confidential`)
3. Create roles: `admin`, `developer`, `viewer`
4. Assign roles to users

### 2. Configure the Control Plane

```env
KEYCLOAK_URL=http://localhost:8082
KEYCLOAK_REALM=integration-platform
KEYCLOAK_CLIENT_ID=control-plane
KEYCLOAK_CLIENT_SECRET=your-client-secret
```

### 3. Obtain a Token

```bash
TOKEN=$(curl -s -X POST \
  http://localhost:8082/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" \
  -d "client_secret=your-secret" \
  -d "username=admin@example.com" \
  -d "password=password" \
  -d "grant_type=password" \
  | jq -r .access_token)
```

### 4. Use the Token

```bash
curl -H "Authorization: Bearer $TOKEN" http://localhost:8081/flows
```

## Client Credentials (Machine-to-Machine)

For service accounts:

```bash
TOKEN=$(curl -s -X POST \
  http://localhost:8082/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=my-service" \
  -d "client_secret=my-service-secret" \
  -d "grant_type=client_credentials" \
  | jq -r .access_token)
```

## User Invitation

Admins can invite users via the Control Plane (sends an invitation email through Keycloak):

```bash
curl -X POST http://localhost:8081/users/invite \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{ "email": "dev@example.com", "role": "developer" }'
```
