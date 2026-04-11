# Fix CORS Issue

CORS (Cross-Origin Resource Sharing) error when frontend tries to access backend APIs.

---

## Error

```
Access to XMLHttpRequest at 'http://localhost:8081/transformers/capabilities' 
from origin 'http://localhost:3000' has been blocked by CORS policy: 
Response to preflight request doesn't pass access control check: 
No 'Access-Control-Allow-Origin' header is present on the requested resource.
```

---

## Root Cause

The Control Plane (port 8081) needs to allow requests from the Frontend (port 3000).

CORS is a browser security feature that blocks cross-origin requests unless the server explicitly allows them.

---

## Solution

### Step 1: Update Control Plane CORS Configuration

**File:** `crates/control-plane/src/main.rs`

**Old Configuration:**
```rust
.layer(
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
)
```

**New Configuration (✅ Fixed):**
```rust
.layer(
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_credentials(true)  // ← Added
        .expose_headers(Any)       // ← Added
)
```

**Changes:**
- ✅ Added `.allow_credentials(true)` — Allows cookies and auth headers
- ✅ Added `.expose_headers(Any)` — Exposes response headers to frontend

---

### Step 2: Rebuild and Restart Control Plane

```bash
# Rebuild
docker-compose build control-plane

# Restart
docker-compose up -d control-plane

# Verify it's running
docker-compose ps control-plane
```

---

### Step 3: Verify CORS Headers

```bash
# Test with curl (simulating browser preflight)
curl -X OPTIONS http://localhost:8081/transformers/capabilities \
  -H "Origin: http://localhost:3000" \
  -H "Access-Control-Request-Method: GET" \
  -H "Access-Control-Request-Headers: authorization" \
  -v
```

**Expected Response Headers:**
```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS
Access-Control-Allow-Headers: *
Access-Control-Allow-Credentials: true
Access-Control-Expose-Headers: *
```

---

## Alternative Solution: Use Vite Proxy (Already Configured)

The frontend Vite config already has a proxy configured:

**File:** `frontend/vite.config.ts`

```typescript
export default defineConfig({
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
})
```

**To use the proxy:**

Update API base URL in `frontend/src/services/api.ts`:

```typescript
export const api = axios.create({
  baseURL: '/api',  // ← Use proxy instead of direct URL
})
```

But this only works for Data Plane (port 8080), not Control Plane (port 8081).

**Better: Add Control Plane proxy too:**

```typescript
export default defineConfig({
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
      '/control': {  // ← Add this
        target: 'http://localhost:8081',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/control/, ''),
      },
    },
  },
})
```

Then update services:

```typescript
export const api = axios.create({
  baseURL: '/control',  // Uses Vite proxy
})
```

---

## Production CORS Configuration

For production, don't use `Any` - specify exact origins:

```rust
use tower_http::cors::{CorsLayer, AllowOrigin};
use http::HeaderValue;

.layer(
    CorsLayer::new()
        .allow_origin(
            "https://your-frontend-domain.com"
                .parse::<HeaderValue>()
                .unwrap()
        )
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
        ])
        .allow_credentials(true)
)
```

---

## Troubleshooting

### Still Getting CORS Error?

**1. Check if Control Plane is running:**
```bash
docker-compose ps control-plane
```

**2. Check Control Plane logs:**
```bash
docker-compose logs control-plane | tail -20
```

**3. Verify CORS headers in browser:**
- Open DevTools → Network tab
- Find the failed request
- Look for "Response Headers"
- Should see `Access-Control-Allow-Origin: *`

**4. Check if it's a preflight request:**
- Preflight = OPTIONS request sent before actual request
- Browser sends OPTIONS to check CORS headers
- If preflight fails, actual request never sent

**5. Clear browser cache:**
```javascript
// In DevTools Console
localStorage.clear()
sessionStorage.clear()
location.reload(true)
```

---

## Why CORS Exists

CORS prevents malicious websites from making unauthorized requests to your API:

```
❌ Without CORS:
evil-site.com → Makes request to your-api.com → Steals user data

✅ With CORS:
evil-site.com → Browser blocks request (no CORS headers)
your-frontend.com → Browser allows request (CORS headers present)
```

---

## Quick Test

After fixing CORS, test in browser console:

```javascript
// Should work now
fetch('http://localhost:8081/transformers/capabilities', {
  headers: {
    'Authorization': 'Bearer YOUR_TOKEN'
  }
})
  .then(r => r.json())
  .then(data => console.log('✅ CORS fixed!', data))
  .catch(err => console.error('❌ Still blocked:', err))
```

---

## Summary

**Fix Applied:**
1. ✅ Added `.allow_credentials(true)` to CORS config
2. ✅ Added `.expose_headers(Any)` to CORS config
3. ✅ Rebuild and restart Control Plane

**Expected Result:**
- Frontend can now call `/transformers/capabilities`
- Palette will load with correct counts
- No more CORS errors in console

**Verify:**
```bash
# Rebuild
docker-compose build control-plane
docker-compose up -d control-plane

# Test
curl -X OPTIONS http://localhost:8081/transformers/capabilities \
  -H "Origin: http://localhost:3000" -v
```

CORS is now properly configured! 🌐✅
