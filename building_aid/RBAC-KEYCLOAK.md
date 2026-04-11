# Role-Based Access Control (RBAC) with Keycloak

Complete RBAC implementation with Keycloak authentication for the Control Plane.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        User/Client                                │
└───────────────────────────┬──────────────────────────────────────┘
                            │
                            │ 1. Login with username/password
                            ▼
┌──────────────────────────────────────────────────────────────────┐
│                   Keycloak :8080/auth                            │
│                                                                  │
│  • User authentication                                           │
│  • Role management (admin, developer, viewer)                    │
│  • JWT token issuance (RS256)                                    │
│  • User invitation & verification                                │
└───────────────────────────┬──────────────────────────────────────┘
                            │
                            │ 2. Returns JWT token
                            ▼
                       User stores token
                            │
                            │ 3. API request with Bearer token
                            ▼
┌──────────────────────────────────────────────────────────────────┐
│              Control Plane :8081 (RBAC Protected)                │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  RBAC Middleware Pipeline                                  │ │
│  │                                                            │ │
│  │  1. rbac_middleware:                                       │ │
│  │     • Extract Bearer token                                 │ │
│  │     • Validate JWT with Keycloak public key               │ │
│  │     • Extract user info & roles from claims               │ │
│  │     • Store User in request extensions                    │ │
│  │                                                            │ │
│  │  2. permission_middleware:                                 │ │
│  │     • Extract required permission from path + method      │ │
│  │     • Check: user.can(required_permission)                │ │
│  │     • Allow or 403 Forbidden                              │ │
│  └────────────────────────────────────────────────────────────┘ │
│                            ↓                                     │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Endpoint Handler (if authorized)                          │ │
│  │  • Create flow                                             │ │
│  │  • Create connector                                        │ │
│  │  • Invite user (admin only)                                │ │
│  └────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

---

## Roles & Permissions

### Admin Role

**Full access** to all Control Plane features + user management.

| Permission | Description |
|------------|-------------|
| **User Management** | ✅ Invite users, delete users, list users |
| **Flows** | ✅ Create, read, update, delete |
| **Connectors** | ✅ Create, read, update, delete |
| **APIs** | ✅ Create, read, update, delete |
| **Clients** | ✅ Create, read, update, delete |
| **Metrics** | ✅ Read |
| **Rate Limits** | ✅ Read |

**Admin-only endpoints:**
- `POST /users/invite`
- `GET /users`
- `DELETE /users/:user_id`

---

### Developer Role

**Read/write** access to integration resources (flows, connectors, clients).

| Permission | Description |
|------------|-------------|
| **User Management** | ❌ Cannot invite or manage users |
| **Flows** | ✅ Create, read, update, delete |
| **Connectors** | ✅ Create, read, update, delete |
| **APIs** | ✅ Create, read, update, delete |
| **Clients** | ✅ Create, read, update, delete |
| **Metrics** | ✅ Read |
| **Rate Limits** | ✅ Read |

**Can manage:**
- Flows (CRUD)
- Connector instances (CRUD)
- API definitions (CRUD)
- Auth clients (CRUD)

**Cannot:**
- Invite users
- Delete users
- Manage user roles

---

### Viewer Role

**Read-only** access to monitoring and configurations.

| Permission | Description |
|------------|-------------|
| **User Management** | ❌ Read-only (cannot modify) |
| **Flows** | 👁️ Read-only |
| **Connectors** | 👁️ Read-only |
| **APIs** | 👁️ Read-only |
| **Clients** | 👁️ Read-only |
| **Metrics** | ✅ Read |
| **Rate Limits** | ✅ Read |

**Can view:**
- Flows (list, get)
- Connector instances (list, get)
- APIs (list, get)
- Clients (list, get)
- Metrics
- Rate limit statistics

**Cannot:**
- Create, update, or delete anything
- Invite users

---

## Keycloak Setup

### 1. Install Keycloak

**Docker Compose:**
```yaml
services:
  keycloak:
    image: quay.io/keycloak/keycloak:23.0
    container_name: integration-keycloak
    environment:
      KEYCLOAK_ADMIN: admin
      KEYCLOAK_ADMIN_PASSWORD: admin123
      KC_HOSTNAME: localhost
      KC_HTTP_ENABLED: "true"
      KC_HTTP_PORT: "8080"
    ports:
      - "8180:8080"  # Keycloak on 8180 to avoid conflict with data-plane
    command: start-dev
    networks:
      - integration-network
```

```bash
docker-compose up -d keycloak
```

---

### 2. Configure Realm

1. **Access Keycloak Admin Console:**
   - URL: `http://localhost:8180/auth/admin`
   - Username: `admin`
   - Password: `admin123`

2. **Create Realm:**
   - Click "Create realm"
   - Name: `integration-platform`
   - Enable: Yes
   - Save

3. **Create Client:**
   - Clients → Create client
   - Client ID: `control-plane`
   - Client authentication: ON
   - Valid redirect URIs: `http://localhost:8081/*`
   - Save
   - Go to "Credentials" tab
   - Copy "Client secret" (save for later)

4. **Create Roles:**
   - Realm roles → Create role
   - Create three roles:
     - `admin`
     - `developer`
     - `viewer`

5. **Create Initial Admin User:**
   - Users → Create user
   - Username: `admin-user`
   - Email: `admin@example.com`
   - Email verified: ON
   - Save
   - Go to "Credentials" tab → Set password
   - Go to "Role mapping" tab → Assign roles → Select `admin` role

---

### 3. Configure Control Plane

**Environment Variables:**
```bash
# Keycloak Configuration
KEYCLOAK_SERVER_URL=http://keycloak:8080
KEYCLOAK_REALM=integration-platform
KEYCLOAK_CLIENT_ID=control-plane
KEYCLOAK_CLIENT_SECRET=<client-secret-from-keycloak>

# Existing variables
DATABASE_URL=postgresql://...
REDIS_URL=redis://...
NATS_URL=nats://...
JWT_SECRET=...
ENCRYPTION_KEY=...
```

**docker-compose.yml:**
```yaml
services:
  control-plane:
    environment:
      # ... existing vars ...
      KEYCLOAK_SERVER_URL: http://keycloak:8080
      KEYCLOAK_REALM: integration-platform
      KEYCLOAK_CLIENT_ID: control-plane
      KEYCLOAK_CLIENT_SECRET: ${KEYCLOAK_CLIENT_SECRET}
```

---

### 4. Database Migration

Add user invitations table:

```sql
CREATE TABLE IF NOT EXISTS user_invitations (
    id              VARCHAR(64)  PRIMARY KEY,
    email           VARCHAR(255) NOT NULL,
    role            VARCHAR(50)  NOT NULL,
    invited_by      VARCHAR(64)  NOT NULL,
    invited_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ  NOT NULL,
    token           VARCHAR(64)  NOT NULL UNIQUE,
    accepted        BOOLEAN      NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_invitations_email ON user_invitations(email);
CREATE INDEX idx_invitations_token ON user_invitations(token);
```

---

## Usage

### 1. Login & Get Token

**Request:**
```bash
curl -X POST http://localhost:8180/auth/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" \
  -d "client_secret=<client-secret>" \
  -d "username=admin-user" \
  -d "password=<password>" \
  -d "grant_type=password"
```

**Response:**
```json
{
  "access_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_in": 300,
  "refresh_token": "...",
  "token_type": "Bearer"
}
```

Save the `access_token` for API requests.

---

### 2. Use Token for API Requests

**All Control Plane requests now require Bearer token:**

```bash
TOKEN="eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9..."

# Create flow (requires Developer or Admin role)
curl -X POST http://localhost:8081/flows \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "my-flow",
    "name": "My Flow",
    "trigger": {"type": "http", "path": "/test", "method": "GET"},
    "steps": [...]
  }'

# List flows (all roles can read)
curl http://localhost:8081/flows \
  -H "Authorization: Bearer $TOKEN"

# Delete flow (requires Developer or Admin role)
curl -X DELETE http://localhost:8081/flows/my-flow \
  -H "Authorization: Bearer $TOKEN"
```

---

### 3. Invite New User (Admin Only)

**Request:**
```bash
curl -X POST http://localhost:8081/users/invite \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "developer@example.com",
    "role": "developer"
  }'
```

**Response:**
```json
{
  "invitation_id": "inv_abc123",
  "email": "developer@example.com",
  "role": "developer",
  "keycloak_user_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "expires_at": "2024-02-25T10:00:00Z",
  "status": "invitation_sent"
}
```

**What happens:**
1. User created in Keycloak with role `developer`
2. Invitation stored in database
3. Email sent to user (via Keycloak) with verification link
4. User must verify email and set password
5. User can then login and get token

---

### 4. List Users (Admin Only)

**Request:**
```bash
curl http://localhost:8081/users \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "users": [
    {
      "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
      "username": "admin-user",
      "email": "admin@example.com",
      "roles": ["admin"],
      "name": "Admin User"
    },
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "developer",
      "email": "developer@example.com",
      "roles": ["developer"],
      "name": null
    }
  ],
  "count": 2
}
```

---

### 5. Get Current User Info

**Request:**
```bash
curl http://localhost:8081/users/me \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "username": "admin-user",
  "email": "admin@example.com",
  "name": "Admin User",
  "roles": ["admin"]
}
```

---

## Permission Matrix

| Endpoint | Method | Admin | Developer | Viewer |
|----------|--------|-------|-----------|--------|
| **Users** |
| `POST /users/invite` | POST | ✅ | ❌ | ❌ |
| `GET /users` | GET | ✅ | ❌ | ❌ |
| `DELETE /users/:id` | DELETE | ✅ | ❌ | ❌ |
| `GET /users/me` | GET | ✅ | ✅ | ✅ |
| **Flows** |
| `GET /flows` | GET | ✅ | ✅ | ✅ |
| `POST /flows` | POST | ✅ | ✅ | ❌ |
| `PUT /flows/:id` | PUT | ✅ | ✅ | ❌ |
| `DELETE /flows/:id` | DELETE | ✅ | ✅ | ❌ |
| **Connectors** |
| `GET /connector-instances` | GET | ✅ | ✅ | ✅ |
| `POST /connector-instances` | POST | ✅ | ✅ | ❌ |
| `DELETE /connector-instances/:id` | DELETE | ✅ | ✅ | ❌ |
| **APIs** |
| `GET /apis` | GET | ✅ | ✅ | ✅ |
| `POST /apis` | POST | ✅ | ✅ | ❌ |
| `DELETE /apis/:id` | DELETE | ✅ | ✅ | ❌ |
| **Clients** |
| `GET /auth/clients` | GET | ✅ | ✅ | ✅ |
| `POST /auth/clients` | POST | ✅ | ✅ | ❌ |
| `DELETE /auth/clients/:id` | DELETE | ✅ | ✅ | ❌ |
| **Monitoring** |
| `GET /metrics` | GET | ✅ | ✅ | ✅ |
| `GET /rate-limit-stats` | GET | ✅ | ✅ | ✅ |
| **Public** |
| `GET /health` | GET | ✅ | ✅ | ✅ |

---

## Error Responses

### 401 Unauthorized
**Missing or invalid token:**
```json
{
  "error": "Missing or invalid Authorization header",
  "hint": "Use: Authorization: Bearer <keycloak-token>"
}
```

**Expired token:**
```json
{
  "error": "Invalid or expired token",
  "details": "Token expired at 2024-02-18T10:00:00Z"
}
```

---

### 403 Forbidden
**Insufficient permissions:**
```json
{
  "error": "Insufficient permissions",
  "required": "WriteFlows",
  "your_roles": ["viewer"],
  "hint": "Contact admin to request access"
}
```

---

## JWT Token Structure

**Keycloak RS256 JWT claims:**
```json
{
  "sub": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "preferred_username": "admin-user",
  "email": "admin@example.com",
  "name": "Admin User",
  "realm_access": {
    "roles": ["admin", "developer"]
  },
  "exp": 1708257600,
  "iat": 1708257300,
  "aud": "control-plane"
}
```

**Platform extracts:**
- `sub` → User ID
- `preferred_username` → Username
- `email` → Email
- `realm_access.roles` → User roles (admin/developer/viewer)

---

## Testing

### 1. Create Test Users

```bash
# Admin user (already created in setup)
# username: admin-user, role: admin

# Create developer
curl -X POST http://localhost:8081/users/invite \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"email":"dev@example.com","role":"developer"}'

# Create viewer
curl -X POST http://localhost:8081/users/invite \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"email":"viewer@example.com","role":"viewer"}'
```

### 2. Test Role Permissions

**Admin can do everything:**
```bash
# Login as admin
ADMIN_TOKEN=$(curl -X POST http://localhost:8180/auth/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" -d "client_secret=$CLIENT_SECRET" \
  -d "username=admin-user" -d "password=admin123" -d "grant_type=password" | jq -r '.access_token')

# Create flow (should succeed)
curl -X POST http://localhost:8081/flows -H "Authorization: Bearer $ADMIN_TOKEN" -d '{...}'

# Invite user (should succeed)
curl -X POST http://localhost:8081/users/invite -H "Authorization: Bearer $ADMIN_TOKEN" -d '{...}'
```

**Developer can manage integrations:**
```bash
# Login as developer
DEV_TOKEN=$(curl -X POST http://localhost:8180/auth/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" -d "client_secret=$CLIENT_SECRET" \
  -d "username=developer" -d "password=dev123" -d "grant_type=password" | jq -r '.access_token')

# Create flow (should succeed)
curl -X POST http://localhost:8081/flows -H "Authorization: Bearer $DEV_TOKEN" -d '{...}'

# Invite user (should fail - 403 Forbidden)
curl -X POST http://localhost:8081/users/invite -H "Authorization: Bearer $DEV_TOKEN" -d '{...}'
```

**Viewer can only read:**
```bash
# Login as viewer
VIEWER_TOKEN=$(curl -X POST http://localhost:8180/auth/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" -d "client_secret=$CLIENT_SECRET" \
  -d "username=viewer" -d "password=viewer123" -d "grant_type=password" | jq -r '.access_token')

# List flows (should succeed)
curl http://localhost:8081/flows -H "Authorization: Bearer $VIEWER_TOKEN"

# Create flow (should fail - 403 Forbidden)
curl -X POST http://localhost:8081/flows -H "Authorization: Bearer $VIEWER_TOKEN" -d '{...}'
```

---

## Migration from JWT-only Auth

**Before (JWT-only):**
- Control Plane issued JWTs directly
- No role-based access control
- All authenticated users had full access

**After (Keycloak RBAC):**
- Keycloak manages users and roles
- JWT tokens issued by Keycloak (RS256)
- Fine-grained permission checks

**Backward compatibility:**
- Old JWT auth still works (for Data Plane triggers)
- Control Plane now requires Keycloak tokens
- Data Plane `/api/trigger` continues using client credentials or JWT

---

## Troubleshooting

### Keycloak not reachable
**Error:** `Failed to fetch Keycloak public key`

**Fix:**
```bash
# Check Keycloak is running
docker-compose ps keycloak

# Check logs
docker-compose logs keycloak

# Verify URL is correct
curl http://localhost:8180/auth/realms/integration-platform/.well-known/openid-configuration
```

### Token validation fails
**Error:** `Invalid or expired token`

**Causes:**
- Token expired (default 5 minutes)
- Wrong client_id in audience claim
- Keycloak public key rotated

**Fix:**
```bash
# Get new token
curl -X POST http://localhost:8180/auth/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" \
  -d "client_secret=$CLIENT_SECRET" \
  -d "username=admin-user" \
  -d "password=admin123" \
  -d "grant_type=password"
```

### Role not found in token
**Error:** `Insufficient permissions`

**Fix:**
- Verify role assigned in Keycloak: Users → <user> → Role mapping
- Ensure role name matches exactly: `admin`, `developer`, or `viewer`
- Get new token after role assignment

---

## Production Considerations

### 1. Use HTTPS
- Keycloak should always be behind HTTPS in production
- Use Let's Encrypt or corporate certificates

### 2. Secure Client Secret
- Store `KEYCLOAK_CLIENT_SECRET` in secrets manager (AWS Secrets Manager, Vault)
- Rotate regularly
- Never commit to version control

### 3. Token Expiry
- Default 5 minutes is aggressive for production
- Increase to 1 hour: Keycloak → Realm settings → Tokens → Access Token Lifespan: `3600`
- Use refresh tokens for long-lived sessions

### 4. Email Configuration
- Configure SMTP in Keycloak for invitation emails
- Keycloak → Realm settings → Email
- Set SMTP server, from address, authentication

### 5. Multi-factor Authentication (MFA)
- Enable in Keycloak: Authentication → Required actions → Configure OTP
- Force for admin users

### 6. Audit Logging
- Enable Keycloak event logging
- Ship logs to centralized system (ELK, Splunk)

---

## Summary

| Feature | Implementation |
|---------|----------------|
| **Authentication** | Keycloak OpenID Connect (RS256 JWT) |
| **Roles** | Admin, Developer, Viewer |
| **User Management** | Keycloak Admin API |
| **Invitation** | Admin-initiated, email verification |
| **Token Validation** | RS256 with Keycloak public key |
| **Permission Check** | Middleware-based, per-endpoint |
| **Fine-grained Control** | 12 distinct permissions |

**Your Control Plane now has enterprise-grade access control!** 🔐👥✅
