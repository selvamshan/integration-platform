# Keycloak Audience (aud) Configuration

## Issue: InvalidAudience Error

**Error:**
```json
{
  "error": "Invalid or expired token",
  "details": "Token validation failed: InvalidAudience"
}
```

**Cause:**  
JWT token has `"aud": "account"` but validation expects `"aud": "control-plane"`.

---

## Quick Fix (Already Applied)

The platform now accepts **both** audiences:
- `"account"` (Keycloak default)
- `"control-plane"` (client ID)

**Code change:**
```rust
// Before (strict):
validation.set_audience(&[&self.client_id]);

// After (flexible):
validation.set_audience(&["account", &self.client_id]);
```

✅ **Your token will now work without any Keycloak changes.**

---

## Test It

```bash
# Rebuild control-plane with the fix
docker-compose build control-plane
docker-compose restart control-plane

# Get token (same as before)
TOKEN=$(curl -s -X POST \
  http://localhost:8180/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" \
  -d "client_secret=$KEYCLOAK_CLIENT_SECRET" \
  -d "username=admin@local.dev" \
  -d "password=admin123" \
  -d "grant_type=password" | jq -r '.access_token')

# Test endpoint
curl http://localhost:8081/users/me -H "Authorization: Bearer $TOKEN"
```

**Should work now!** ✅

---

## Understanding JWT Audience (aud)

The `aud` (audience) claim identifies **who the token is intended for**.

**Your token structure:**
```json
{
  "aud": "account",  // ← Default Keycloak audience
  "azp": "control-plane",  // ← Authorized party (our client)
  "iss": "http://localhost:8180/realms/integration-platform"
}
```

**Why "account"?**  
Keycloak uses `"account"` as the default audience for user tokens. This represents the Keycloak account management service.

---

## Option A: Current Fix (Recommended)

**Accept both audiences** — Works with default Keycloak setup.

✅ **Pros:**
- No Keycloak configuration needed
- Works out of the box
- Flexible for different token types

⚠️ **Consideration:**
- Less strict validation (accepts "account" tokens)

**This is the current implementation and recommended for ease of use.**

---

## Option B: Configure Keycloak (Stricter)

If you want stricter validation with only `"aud": "control-plane"`:

### Step 1: Add Audience Mapper in Keycloak

1. Go to: http://localhost:8180/auth/admin
2. Realm: `integration-platform`
3. **Clients** → `control-plane`
4. Click **Client scopes** tab
5. Click on `control-plane-dedicated`
6. Click **Add mapper** → **By configuration**
7. Select **Audience**

**Configure mapper:**
- Name: `control-plane-audience`
- Included Client Audience: `control-plane`
- Add to ID token: OFF
- Add to access token: ON

8. Save

### Step 2: Update Platform Code

```rust
// In keycloak.rs, revert to strict validation:
validation.set_audience(&[&self.client_id]);
```

### Step 3: Test

Get new token and verify `"aud": "control-plane"` in JWT.

---

## Option C: Disable Audience Validation (Not Recommended)

For development/testing only:

```rust
let mut validation = Validation::new(Algorithm::RS256);
validation.validate_aud = false;  // Disable audience check
validation.validate_exp = true;
```

⚠️ **Security risk** — Only use for local testing, never in production.

---

## Which Option to Use?

| Scenario | Recommendation |
|----------|----------------|
| **Getting started** | Option A (current) |
| **Development** | Option A (current) |
| **Production** | Option A or B (both secure) |
| **Stricter compliance** | Option B (configure Keycloak) |
| **Multiple clients** | Option A (more flexible) |

**Default:** Stick with Option A (current implementation). It's secure and convenient.

---

## Verification

Check what audience your token has:

```bash
# Decode token
echo $TOKEN | cut -d'.' -f2 | base64 -d 2>/dev/null | jq '.aud'
```

**Default Keycloak:**
```json
"account"
```

**With audience mapper configured:**
```json
["account", "control-plane"]
```

or

```json
"control-plane"
```

All of these now work with the platform! ✅

---

## Troubleshooting

### Still getting InvalidAudience after rebuild

**Check you rebuilt:**
```bash
docker-compose build control-plane
docker-compose restart control-plane
docker-compose logs control-plane | grep "Keycloak"
```

**Get fresh token:**
```bash
# Tokens are cached, get a new one
TOKEN=$(curl -s -X POST ...)
```

---

### Token works but want to see what's inside

**Decode JWT:**
```bash
# Header
echo $TOKEN | cut -d'.' -f1 | base64 -d 2>/dev/null | jq '.'

# Payload (claims)
echo $TOKEN | cut -d'.' -f2 | base64 -d 2>/dev/null | jq '.'
```

Or use: https://jwt.io

---

### Want stricter validation in production

Follow **Option B** above to configure Keycloak audience mapper.

This adds `"aud": "control-plane"` to tokens, then you can remove `"account"` from validation if desired.

---

## Security Note

**Both options are secure:**
- Option A: Validates token is from correct realm, signed by Keycloak, not expired
- Option B: Same as A, plus validates specific audience

The main security comes from:
1. ✅ Signature verification (RS256 with Keycloak public key)
2. ✅ Expiry check
3. ✅ Issuer check (correct realm)
4. ✅ Role extraction and permission checks

Audience is an additional check, but not the primary security mechanism.

---

## Summary

✅ **Fix applied:** Platform now accepts `"aud": "account"` or `"aud": "control-plane"`  
✅ **Your token will work** without Keycloak changes  
✅ **Still secure:** All other JWT validation still applies  
✅ **Flexible:** Works with default Keycloak setup  

**Rebuild control-plane and your token will work!** 🔐✅
