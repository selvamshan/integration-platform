# Keycloak Role Setup — Client Roles vs Realm Roles

Quick guide on setting up roles in Keycloak for the integration platform.

---

## Role Types in Keycloak

Keycloak supports two types of roles:

1. **Realm Roles** — Global roles for the entire realm
2. **Client Roles** — Specific to a client application

**The platform supports BOTH!** Roles can be configured either way.

---

## Option 1: Client Roles (Recommended)

Client roles are scoped to the `control-plane` client and are better for multi-application setups.

### Setup Steps

**1. Create Client Roles:**

1. Go to Keycloak Admin Console: http://localhost:8180/auth/admin
2. Select realm: `integration-platform`
3. Go to: **Clients** → `control-plane`
4. Click **Roles** tab
5. Click **Create role**
6. Create three roles:
   - Role name: `admin`
   - Role name: `developer`
   - Role name: `viewer`

**2. Assign Client Role to User:**

1. Go to: **Users** → Select user (e.g., `admin@local.dev`)
2. Click **Role mapping** tab
3. Click **Assign role**
4. Filter: **Filter by clients**
5. Select client: `control-plane`
6. Check: `admin` (or `developer`, `viewer`)
7. Click **Assign**

**JWT Token Structure (Client Roles):**
```json
{
  "resource_access": {
    "control-plane": {
      "roles": ["admin"]
    }
  }
}
```

✅ **Platform now extracts roles from here!**

---

## Option 2: Realm Roles

Realm roles are global and simpler for single-application setups.

### Setup Steps

**1. Create Realm Roles:**

1. Go to Keycloak Admin Console
2. Select realm: `integration-platform`
3. Go to: **Realm roles**
4. Click **Create role**
5. Create three roles:
   - Role name: `admin`
   - Role name: `developer`  
   - Role name: `viewer`

**2. Assign Realm Role to User:**

1. Go to: **Users** → Select user
2. Click **Role mapping** tab
3. Click **Assign role**
4. Check: `admin` (or `developer`, `viewer`)
5. Click **Assign**

**JWT Token Structure (Realm Roles):**
```json
{
  "realm_access": {
    "roles": [
      "admin",
      "default-roles-integration-platform",
      "offline_access"
    ]
  }
}
```

✅ **Platform also extracts roles from here!**

---

## Which Should I Use?

| Scenario | Recommendation |
|----------|----------------|
| **Single application** | Either works — Realm roles are simpler |
| **Multiple applications** | Client roles (better isolation) |
| **Already have realm roles** | Keep using them (both work) |
| **New setup** | Client roles (more scalable) |

**Both work!** The platform checks both locations and merges roles.

---

## Verifying Role Assignment

### 1. Get Token

```bash
TOKEN=$(curl -s -X POST \
  http://localhost:8180/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" \
  -d "client_secret=YOUR_CLIENT_SECRET" \
  -d "username=admin@local.dev" \
  -d "password=admin123" \
  -d "grant_type=password" | jq -r '.access_token')
```

### 2. Decode Token

```bash
# Using jwt.io or decode manually
echo $TOKEN | cut -d'.' -f2 | base64 -d 2>/dev/null | jq '.'
```

### 3. Check for Roles

**Look for either:**

**Client roles:**
```json
"resource_access": {
  "control-plane": {
    "roles": ["admin"]  // ← Your role here
  }
}
```

**OR Realm roles:**
```json
"realm_access": {
  "roles": [
    "admin",  // ← Your role here
    "offline_access"
  ]
}
```

### 4. Test with Control Plane

```bash
curl http://localhost:8081/users/me \
  -H "Authorization: Bearer $TOKEN"
```

**Expected response:**
```json
{
  "id": "c94ec8d3-5dee-40b7-997f-833a007fec54",
  "username": "admin@local.dev",
  "email": "admin@local.dev",
  "name": "admin local",
  "roles": ["admin"]  // ← Should show your role
}
```

---

## Troubleshooting

### Role not showing in token

**Check role assignment:**
1. Keycloak → Users → Your user → Role mapping
2. Make sure role is assigned
3. Check both **Assigned roles** and **Effective roles**

### Role in token but platform says "Insufficient permissions"

**Check role name matches exactly:**
- Platform expects: `admin`, `developer`, or `viewer` (lowercase)
- Case-sensitive!
- Check spelling

**Get new token:**
```bash
# Role changes require new token
TOKEN=$(curl -s -X POST ...)
```

### Both realm and client roles assigned

**No problem!** Platform merges both:
- If you have `admin` in realm roles AND `developer` in client roles
- Platform gives you: `["admin", "developer"]`
- You get the union of all permissions

---

## Migration: Realm → Client Roles

Already using realm roles and want to switch?

**No migration needed!** Both work. But if you want to move:

1. **Assign client roles** (as described above)
2. **Test** — verify new roles work
3. **Remove realm roles** — Users → Role mapping → Unassign realm role

Users keep access during transition (both roles work simultaneously).

---

## Default Roles

**Keycloak includes default roles** you can ignore:
- `default-roles-integration-platform`
- `offline_access`
- `uma_authorization`

Platform filters these out automatically (not recognized as admin/developer/viewer).

---

## Quick Reference

| Task | Location |
|------|----------|
| **Create client role** | Clients → control-plane → Roles → Create role |
| **Create realm role** | Realm roles → Create role |
| **Assign client role** | Users → User → Role mapping → Assign role → Filter by clients |
| **Assign realm role** | Users → User → Role mapping → Assign role |
| **View user roles** | Users → User → Role mapping |
| **Decode JWT** | https://jwt.io or `base64 -d` |

---

## Summary

✅ **Client roles** are in `resource_access.control-plane.roles`  
✅ **Realm roles** are in `realm_access.roles`  
✅ **Platform checks both** and merges roles  
✅ **Both work** — choose what fits your setup  

Your token has `resource_access.control-plane.roles: ["admin"]` — this will now work correctly! 🔐✅
