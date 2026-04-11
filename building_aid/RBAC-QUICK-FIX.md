# RBAC Quick Fix — "No user context" Error

Getting `{"error":"No user context"}` when calling `/users/me` or other endpoints?

---

## Problem

```json
{"error": "No user context"}
```

or

```json
{"error": "Authentication required (enable RBAC)"}
```

---

## Cause

**RBAC middleware is disabled by default.** The middleware that validates tokens and extracts user info is commented out.

---

## Solution — 3 Steps

### Step 1: Enable RBAC Middleware

**Option A: Use the script (easiest):**
```bash
cd integration-platform
./enable-rbac.sh
```

**Option B: Manual edit:**

Edit `crates/control-plane/src/main.rs` (around line 177-180):

**Before (disabled):**
```rust
// ── RBAC Middleware (comment out to disable) ──────────────────────
// Uncomment these lines to enable Keycloak-based RBAC:
// .layer(middleware::from_fn(permission_middleware))
// .layer(middleware::from_fn_with_state(keycloak.clone(), rbac_middleware))
.layer(TraceLayer::new_for_http())
```

**After (enabled):**
```rust
// ── RBAC Middleware ───────────────────────────────────────────────
.layer(middleware::from_fn(permission_middleware))
.layer(middleware::from_fn_with_state(keycloak.clone(), rbac_middleware))
.layer(TraceLayer::new_for_http())
```

**Just remove the `//` from those two lines.**

---

### Step 2: Set Keycloak Client Secret

```bash
export KEYCLOAK_CLIENT_SECRET="your-client-secret-from-keycloak"
```

To get the client secret:
1. Go to http://localhost:8180/auth/admin
2. Login: admin / admin123
3. Realm: integration-platform
4. Clients → control-plane
5. Credentials tab → Copy client secret

Or set in `docker-compose.yml`:
```yaml
control-plane:
  environment:
    KEYCLOAK_CLIENT_SECRET: "paste-secret-here"
```

---

### Step 3: Rebuild & Restart

```bash
docker-compose build control-plane
docker-compose restart control-plane
```

Wait for control-plane to be ready:
```bash
docker-compose logs -f control-plane | grep "listening"
```

---

## Test

**Get token from Keycloak:**
```bash
TOKEN=$(curl -s -X POST \
  http://localhost:8180/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" \
  -d "client_secret=$KEYCLOAK_CLIENT_SECRET" \
  -d "username=admin@local.dev" \
  -d "password=admin123" \
  -d "grant_type=password" | jq -r '.access_token')

echo "Token: ${TOKEN:0:50}..."
```

**Test endpoint:**
```bash
curl http://localhost:8081/users/me \
  -H "Authorization: Bearer $TOKEN"
```

**Expected (success):**
```json
{
  "id": "c94ec8d3-5dee-40b7-997f-833a007fec54",
  "username": "admin@local.dev",
  "email": "admin@local.dev",
  "name": "admin local",
  "roles": ["admin"]
}
```

**If you still get errors, see troubleshooting below.**

---

## Troubleshooting

### Still getting "No user context"

**Check logs:**
```bash
docker-compose logs control-plane | tail -50
```

**Look for:**
- ✅ `"✅ Keycloak integration enabled"` — Good!
- ❌ `"⚠️ Keycloak not configured"` — Need to set client secret
- ❌ No RBAC logs at all — Middleware still commented out

---

### "Missing or invalid Authorization header"

**You forgot the Bearer prefix:**
```bash
# Wrong:
curl http://localhost:8081/users/me -H "Authorization: $TOKEN"

# Right:
curl http://localhost:8081/users/me -H "Authorization: Bearer $TOKEN"
```

---

### "Invalid or expired token"

**Token expired (default 5 minutes):**
```bash
# Get fresh token
TOKEN=$(curl -s -X POST ...)
```

**Client secret wrong:**
```bash
# Verify client secret in Keycloak
# Clients → control-plane → Credentials tab
```

**Wrong realm or client_id:**
```bash
# Check environment variables
docker-compose exec control-plane env | grep KEYCLOAK
```

---

### "Failed to fetch Keycloak public key"

**Keycloak not reachable from control-plane:**
```bash
# Test connectivity
docker-compose exec control-plane curl http://keycloak:8080/realms/integration-platform/.well-known/openid-configuration

# Should return JSON config
```

**If fails:**
- Check Keycloak is running: `docker-compose ps keycloak`
- Check logs: `docker-compose logs keycloak`

---

### "Insufficient permissions"

**Token is valid but lacks required role:**

Check what roles you have:
```bash
curl http://localhost:8081/users/me -H "Authorization: Bearer $TOKEN" | jq '.roles'
```

**To access admin endpoints, you need `admin` role:**
1. Keycloak → Users → Your user → Role mapping
2. Assign role: `admin` (client or realm role)
3. Get new token

See `KEYCLOAK-ROLE-SETUP.md` for role configuration.

---

## Verification Checklist

Run through this checklist:

```bash
# 1. RBAC middleware enabled?
grep -A1 "RBAC Middleware" crates/control-plane/src/main.rs | grep -v "//"
# Should show: .layer(middleware::from_fn(permission_middleware))
#              .layer(middleware::from_fn_with_state(keycloak.clone(), rbac_middleware))

# 2. Client secret set?
docker-compose exec control-plane env | grep KEYCLOAK_CLIENT_SECRET
# Should show: KEYCLOAK_CLIENT_SECRET=<some-value>

# 3. Keycloak running?
docker-compose ps keycloak
# Should show: Up

# 4. Can get token?
curl -s -X POST http://localhost:8180/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" \
  -d "client_secret=$KEYCLOAK_CLIENT_SECRET" \
  -d "username=admin@local.dev" \
  -d "password=admin123" \
  -d "grant_type=password" | jq -r '.access_token'
# Should show: eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...

# 5. Can use token?
TOKEN="paste-token-here"
curl http://localhost:8081/users/me -H "Authorization: Bearer $TOKEN"
# Should show: {"id":"...","username":"...","roles":["admin"]}
```

If all 5 pass, RBAC is working! ✅

---

## Quick Commands

**Enable RBAC:**
```bash
./enable-rbac.sh
docker-compose build control-plane
docker-compose restart control-plane
```

**Get token:**
```bash
TOKEN=$(curl -s -X POST \
  http://localhost:8180/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" \
  -d "client_secret=$KEYCLOAK_CLIENT_SECRET" \
  -d "username=admin@local.dev" \
  -d "password=admin123" \
  -d "grant_type=password" | jq -r '.access_token')
```

**Test:**
```bash
curl http://localhost:8081/users/me -H "Authorization: Bearer $TOKEN"
```

---

## Summary

| Error | Cause | Fix |
|-------|-------|-----|
| `"No user context"` | RBAC disabled | Uncomment middleware |
| `"Missing Authorization header"` | No token sent | Add `-H "Authorization: Bearer $TOKEN"` |
| `"Invalid token"` | Token expired/wrong | Get new token |
| `"Insufficient permissions"` | Wrong role | Assign correct role in Keycloak |

**After enabling RBAC, your token will work!** 🔐✅
