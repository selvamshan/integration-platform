# RBAC Setup Guide — Quick Start

This guide shows you how to enable Role-Based Access Control with Keycloak authentication.

---

## Current State

**RBAC is DISABLED by default.**

User management endpoints are registered but:
- No authentication required (for now)
- Keycloak integration is optional
- All existing endpoints work without RBAC

---

## Quick Enable (3 Steps)

### Step 1: Start Keycloak

```bash
# Keycloak is already in docker-compose.yml
docker-compose up -d keycloak

# Wait for it to be ready
docker-compose logs -f keycloak | grep "Running"
```

**Access:** http://localhost:8180/auth/admin  
**Login:** admin / admin123

---

### Step 2: Configure Keycloak

**A. Create Realm:**
1. Click "Create realm"
2. Name: `integration-platform`
3. Save

**B. Create Client:**
1. Clients → Create client
2. Client ID: `control-plane`
3. Client authentication: **ON**
4. Valid redirect URIs: `http://localhost:8081/*`
5. Save
6. Go to **Credentials** tab
7. Copy **Client secret** → save it

**C. Create Roles:**
1. Realm roles → Create role
2. Create: `admin`, `developer`, `viewer`

**D. Create Admin User:**
1. Users → Create user
2. Username: `admin-user`
3. Email: `admin@example.com`
4. Email verified: ON
5. Save
6. Go to **Credentials** tab → Set password (temp off)
7. Go to **Role mapping** → Assign **admin** role

---

### Step 3: Enable RBAC in Control Plane

**A. Set Keycloak Client Secret:**

```bash
# Edit .env or export
export KEYCLOAK_CLIENT_SECRET="<paste-client-secret-here>"
```

Or in `docker-compose.yml`:
```yaml
control-plane:
  environment:
    # ... existing vars ...
    KEYCLOAK_CLIENT_SECRET: "your-client-secret-here"
```

**B. Uncomment RBAC Middleware:**

Edit `crates/control-plane/src/main.rs` (around line 177):

```rust
// BEFORE (RBAC disabled):
        // ── RBAC Middleware (comment out to disable) ──────────────────────
        // Uncomment these lines to enable Keycloak-based RBAC:
        // .layer(middleware::from_fn(permission_middleware))
        // .layer(middleware::from_fn_with_state(keycloak.clone(), rbac_middleware))
        .layer(TraceLayer::new_for_http())
```

```rust
// AFTER (RBAC enabled):
        // ── RBAC Middleware ───────────────────────────────────────────────
        .layer(middleware::from_fn(permission_middleware))
        .layer(middleware::from_fn_with_state(keycloak.clone(), rbac_middleware))
        .layer(TraceLayer::new_for_http())
```

**C. Rebuild and Restart:**

```bash
docker-compose build control-plane
docker-compose restart control-plane
```

---

## Test It

### 1. Get Token

```bash
TOKEN=$(curl -s -X POST \
  http://localhost:8180/auth/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" \
  -d "client_secret=$KEYCLOAK_CLIENT_SECRET" \
  -d "username=admin-user" \
  -d "password=admin123" \
  -d "grant_type=password" | jq -r '.access_token')

echo "Token: $TOKEN"
```

### 2. Use Token

```bash
# Get current user info
curl http://localhost:8081/users/me \
  -H "Authorization: Bearer $TOKEN"

# List users (admin only)
curl http://localhost:8081/users \
  -H "Authorization: Bearer $TOKEN"

# Invite a developer (admin only)
curl -X POST http://localhost:8081/users/invite \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "developer@example.com",
    "role": "developer"
  }'

# Create a flow (admin or developer)
curl -X POST http://localhost:8081/flows \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-flow",
    "name": "Test Flow",
    "trigger": {"type": "http", "path": "/test", "method": "GET"},
    "steps": []
  }'
```

### 3. Test Without Token (Should Fail)

```bash
# Should return 401 Unauthorized
curl http://localhost:8081/flows
```

---

## Verify Setup

```bash
./test-rbac.sh
```

This script checks:
- ✅ Endpoints are registered
- ✅ Keycloak connectivity
- ✅ Token acquisition
- ✅ Authenticated requests

---

## Roles Summary

| Role | Can Do |
|------|--------|
| **Admin** | Everything + invite users |
| **Developer** | Create/update/delete flows, connectors, clients |
| **Viewer** | Read-only access |

---

## Troubleshooting

### "Could not obtain token"

**Check Keycloak is running:**
```bash
docker-compose ps keycloak
curl http://localhost:8180/auth/realms/integration-platform/.well-known/openid-configuration
```

### "Invalid or expired token"

**Check client secret is correct:**
```bash
echo $KEYCLOAK_CLIENT_SECRET
# Should be the secret from Keycloak → Clients → control-plane → Credentials
```

### "No user context"

**RBAC middleware not enabled:**
- Check if you uncommented the middleware lines
- Rebuild: `docker-compose build control-plane`
- Restart: `docker-compose restart control-plane`

---

## Disable RBAC

To disable RBAC (return to open access):

1. **Comment out middleware** in `main.rs`:
   ```rust
   // .layer(middleware::from_fn(permission_middleware))
   // .layer(middleware::from_fn_with_state(keycloak.clone(), rbac_middleware))
   ```

2. **Rebuild:**
   ```bash
   docker-compose build control-plane
   docker-compose restart control-plane
   ```

All endpoints will work without authentication again.

---

## Production Notes

### Required for Production:

1. **HTTPS** — Keycloak must be behind HTTPS
2. **Secrets** — Store `KEYCLOAK_CLIENT_SECRET` in secrets manager
3. **Token Expiry** — Increase from 5min to 1 hour in Keycloak
4. **Email** — Configure SMTP for invitation emails
5. **MFA** — Enable for admin users

See **RBAC-KEYCLOAK.md** for complete production setup.

---

## Summary

**RBAC is optional and disabled by default.**

To enable:
1. ✅ Start Keycloak
2. ✅ Configure realm + client + roles + users
3. ✅ Set `KEYCLOAK_CLIENT_SECRET`
4. ✅ Uncomment 2 lines in `main.rs`
5. ✅ Rebuild + restart

**Your Control Plane will then require Keycloak authentication for all endpoints!**
