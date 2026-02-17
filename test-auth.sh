#!/bin/bash
set -e
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'

CP="http://localhost:8081"
DP="http://localhost:8080"

banner() { echo -e "\n${BLUE}${BOLD}══════════════════════════════════════════${NC}"; echo -e "${BLUE}${BOLD}  $1${NC}"; echo -e "${BLUE}${BOLD}══════════════════════════════════════════${NC}"; }
ok()     { echo -e "${GREEN}✅ $1${NC}"; }
fail()   { echo -e "${RED}❌ $1${NC}"; }
info()   { echo -e "${YELLOW}   $1${NC}"; }

assert_http() {
  local label="$1" expected="$2" actual="$3"
  if [ "$actual" == "$expected" ]; then ok "$label (HTTP $actual)"; else fail "$label — expected $expected got $actual"; fi
}

# ─── Wait for services ────────────────────────────────────────────────────────
banner "0. Waiting for services"
for i in {1..30}; do
  cp_up=$(curl -s -o /dev/null -w "%{http_code}" $CP/health 2>/dev/null)
  dp_up=$(curl -s -o /dev/null -w "%{http_code}" $DP/health 2>/dev/null)
  [ "$cp_up" == "200" ] && [ "$dp_up" == "200" ] && break
  echo -n "."; sleep 2
done
echo ""
[ "$cp_up" == "200" ] && ok "Control Plane up" || { fail "Control Plane unreachable"; exit 1; }
[ "$dp_up" == "200" ] && ok "Data Plane up"    || { fail "Data Plane unreachable";    exit 1; }

# ─── Create a test flow ───────────────────────────────────────────────────────
banner "0b. Create test flow"
curl -s -X POST $CP/flows -H "Content-Type: application/json" -d '{
  "id": "auth-test-flow",
  "name": "Auth Test Flow",
  "trigger": {"type": "http", "path": "/api/trigger/auth-test", "method": "GET"},
  "steps": [{"type": "log", "name": "ok", "message": "Authenticated request!"}]
}' > /dev/null
sleep 2
ok "Flow created: auth-test-flow"

# ─── 1. Unauthenticated requests ─────────────────────────────────────────────
banner "1. Unauthenticated Requests (expect 401)"

code=$(curl -s -o /dev/null -w "%{http_code}" -X POST $DP/flows/auth-test-flow/execute -d '{}')
assert_http "POST /flows/.../execute without auth" "401" "$code"

code=$(curl -s -o /dev/null -w "%{http_code}" $DP/api/trigger/auth-test)
assert_http "GET  /api/trigger/... without auth"  "401" "$code"

# Public endpoints still accessible
code=$(curl -s -o /dev/null -w "%{http_code}" $DP/health)
assert_http "GET  /health (public)" "200" "$code"
code=$(curl -s -o /dev/null -w "%{http_code}" $DP/metrics)
assert_http "GET  /metrics (public)" "200" "$code"

# ─── 2. Create client credential ─────────────────────────────────────────────
banner "2. Create Client Credential (Control Plane)"

RESP=$(curl -s -X POST $CP/auth/clients \
  -H "Content-Type: application/json" \
  -d '{"name": "test-app", "expires_in_days": 30}')

CLIENT_ID=$(echo "$RESP" | jq -r '.client_id')
CLIENT_SECRET=$(echo "$RESP" | jq -r '.client_secret')

if [[ "$CLIENT_ID" == cid_* ]]; then ok "Client created: $CLIENT_ID"
else fail "Client creation failed: $RESP"; exit 1; fi
info "client_id:     $CLIENT_ID"
info "client_secret: ${CLIENT_SECRET:0:10}…  (truncated)"

# ─── 3. Authenticate with Client-Credentials headers ─────────────────────────
banner "3. Method 1 — X-Client-Id / X-Client-Secret Headers"

code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST $DP/flows/auth-test-flow/execute \
  -H "Content-Type: application/json" \
  -H "X-Client-Id: $CLIENT_ID" \
  -H "X-Client-Secret: $CLIENT_SECRET" \
  -d '{}')
assert_http "POST /flows/.../execute with client-creds" "200" "$code"

code=$(curl -s -o /dev/null -w "%{http_code}" \
  $DP/api/trigger/auth-test \
  -H "X-Client-Id: $CLIENT_ID" \
  -H "X-Client-Secret: $CLIENT_SECRET")
assert_http "GET  /api/trigger/... with client-creds"   "200" "$code"

# Wrong secret
code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST $DP/flows/auth-test-flow/execute \
  -H "Content-Type: application/json" \
  -H "X-Client-Id: $CLIENT_ID" \
  -H "X-Client-Secret: wrong_secret" \
  -d '{}')
assert_http "POST /flows/.../execute with wrong secret (expect 401)" "401" "$code"

# ─── 4. Get JWT from Control Plane ───────────────────────────────────────────
banner "4. Issue JWT Token (POST /auth/token)"

TOKEN_RESP=$(curl -s -X POST $CP/auth/token \
  -H "Content-Type: application/json" \
  -d "{\"client_id\":\"$CLIENT_ID\",\"client_secret\":\"$CLIENT_SECRET\"}")

JWT=$(echo "$TOKEN_RESP" | jq -r '.access_token')
TOKEN_TYPE=$(echo "$TOKEN_RESP" | jq -r '.token_type')
EXPIRES=$(echo "$TOKEN_RESP" | jq -r '.expires_in')

if [[ "$JWT" == eyJ* ]]; then ok "JWT issued (${TOKEN_TYPE}, expires in ${EXPIRES}s)"
else fail "Token issuance failed: $TOKEN_RESP"; exit 1; fi
info "token: ${JWT:0:40}… (truncated)"

# ─── 5. Authenticate with JWT Bearer token ───────────────────────────────────
banner "5. Method 2 — Authorization: Bearer <jwt>"

code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST $DP/flows/auth-test-flow/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT" \
  -d '{}')
assert_http "POST /flows/.../execute with JWT" "200" "$code"

code=$(curl -s -o /dev/null -w "%{http_code}" \
  $DP/api/trigger/auth-test \
  -H "Authorization: Bearer $JWT")
assert_http "GET  /api/trigger/... with JWT"   "200" "$code"

# Tampered token
code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST $DP/flows/auth-test-flow/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${JWT}tampered" \
  -d '{}')
assert_http "POST /flows/.../execute with tampered JWT (expect 401)" "401" "$code"

# ─── 6. Client management ────────────────────────────────────────────────────
banner "6. Client Management API"

code=$(curl -s -o /dev/null -w "%{http_code}" $CP/auth/clients)
assert_http "GET /auth/clients"               "200" "$code"

code=$(curl -s -o /dev/null -w "%{http_code}" $CP/auth/clients/$CLIENT_ID)
assert_http "GET /auth/clients/:id"           "200" "$code"

# Deactivate
code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X PATCH $CP/auth/clients/$CLIENT_ID \
  -H "Content-Type: application/json" -d '{"active":false}')
assert_http "PATCH /auth/clients/:id (deactivate)" "200" "$code"

# Deactivated client should be rejected on data-plane
code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST $DP/flows/auth-test-flow/execute \
  -H "Content-Type: application/json" \
  -H "X-Client-Id: $CLIENT_ID" \
  -H "X-Client-Secret: $CLIENT_SECRET" -d '{}')
assert_http "POST /flows with deactivated client (expect 401)" "401" "$code"

# JWT already issued still works (stateless)
code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST $DP/flows/auth-test-flow/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT" -d '{}')
assert_http "POST /flows with JWT while client deactivated (JWT is stateless)" "200" "$code"
info "Note: JWTs remain valid until expiry — revoke by rotating JWT_SECRET"

# Reactivate
curl -s -X PATCH $CP/auth/clients/$CLIENT_ID \
  -H "Content-Type: application/json" -d '{"active":true}' > /dev/null
ok "Client reactivated"

# ─── 7. Delete client ─────────────────────────────────────────────────────────
banner "7. Delete Client"
RESP2=$(curl -s -X POST $CP/auth/clients \
  -H "Content-Type: application/json" -d '{"name":"temp-app"}')
TMP_ID=$(echo "$RESP2" | jq -r '.client_id')
code=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE $CP/auth/clients/$TMP_ID)
assert_http "DELETE /auth/clients/:id" "200" "$code"
code=$(curl -s -o /dev/null -w "%{http_code}" $CP/auth/clients/$TMP_ID)
assert_http "GET after DELETE (expect 404)" "404" "$code"

# ─── 8. Token issuance: wrong secret ─────────────────────────────────────────
banner "8. Token Rejection on Bad Credentials"
code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST $CP/auth/token \
  -H "Content-Type: application/json" \
  -d "{\"client_id\":\"$CLIENT_ID\",\"client_secret\":\"wrong\"}")
assert_http "POST /auth/token with wrong secret (expect 401)" "401" "$code"

code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST $CP/auth/token \
  -H "Content-Type: application/json" \
  -d '{"client_id":"cid_notexist","client_secret":"anything"}')
assert_http "POST /auth/token with unknown client_id (expect 401)" "401" "$code"

# ─── Summary ─────────────────────────────────────────────────────────────────
banner "✅ All Auth Tests Complete"
echo ""
echo -e "${BOLD}Quick reference:${NC}"
echo -e "  Create client:  ${GREEN}POST $CP/auth/clients${NC}"
echo -e "  Get token:      ${GREEN}POST $CP/auth/token${NC}"
echo -e "  Use header:     ${GREEN}X-Client-Id + X-Client-Secret${NC}"
echo -e "  Use JWT:        ${GREEN}Authorization: Bearer <token>${NC}"
