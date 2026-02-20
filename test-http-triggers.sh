#!/bin/bash
set -e
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'

CP="http://localhost:8081"
DP="http://localhost:8080"

banner() { echo -e "\n${BLUE}${BOLD}══════════════════════════════════════════${NC}"; echo -e "${BLUE}${BOLD}  $1${NC}"; echo -e "${BLUE}${BOLD}══════════════════════════════════════════${NC}"; }
ok()     { echo -e "${GREEN}✅ $1${NC}"; }
fail()   { echo -e "${RED}❌ $1${NC}"; exit 1; }
info()   { echo -e "${YELLOW}   $1${NC}"; }

assert_http() {
  local label="$1" expected="$2" actual="$3"
  [ "$actual" == "$expected" ] && ok "$label (HTTP $actual)" || fail "$label — expected $expected got $actual"
}

# ─── Wait for services ────────────────────────────────────────────────────────
banner "0. Waiting for Services"
for i in {1..30}; do
  cp_up=$(curl -s -o /dev/null -w "%{http_code}" $CP/health 2>/dev/null)
  dp_up=$(curl -s -o /dev/null -w "%{http_code}" $DP/health 2>/dev/null)
  [ "$cp_up" == "200" ] && [ "$dp_up" == "200" ] && break
  echo -n "."; sleep 2
done
echo ""
[ "$cp_up" == "200" ] && ok "Control Plane up" || fail "Control Plane unreachable"
[ "$dp_up" == "200" ] && ok "Data Plane up" || fail "Data Plane unreachable"

# ─── Get auth token ───────────────────────────────────────────────────────────
banner "1. Authentication"

# Create client if doesn't exist
CLIENT_RESP=$(curl -s -X POST $CP/auth/clients -H "Content-Type: application/json" -d '{
  "name": "http-trigger-test"
}' 2>/dev/null || echo '{}')

CLIENT_ID=$(echo "$CLIENT_RESP" | jq -r '.client_id // empty')
CLIENT_SECRET=$(echo "$CLIENT_RESP" | jq -r '.client_secret // empty')

if [ -z "$CLIENT_ID" ]; then
  # Client might already exist, use default test credentials
  info "Using existing client credentials"
  # Try to list clients and grab first one
  FIRST_CLIENT=$(curl -s $CP/auth/clients | jq -r '.clients[0].client_id // empty')
  if [ -n "$FIRST_CLIENT" ]; then
    CLIENT_ID="$FIRST_CLIENT"
    info "Found existing client: $CLIENT_ID"
  fi
fi

# For testing, we'll create a fresh one
CLIENT_RESP=$(curl -s -X POST $CP/auth/clients -H "Content-Type: application/json" -d '{"name":"trigger-tester"}')
CLIENT_ID=$(echo "$CLIENT_RESP" | jq -r '.client_id')
CLIENT_SECRET=$(echo "$CLIENT_RESP" | jq -r '.client_secret')

ok "Client created: $CLIENT_ID"

# Get JWT token
TOKEN_RESP=$(curl -s -X POST $CP/auth/token -H "Content-Type: application/json" \
  -d "{\"client_id\":\"$CLIENT_ID\",\"client_secret\":\"$CLIENT_SECRET\"}")
TOKEN=$(echo "$TOKEN_RESP" | jq -r '.access_token')

if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
  fail "Failed to get auth token"
fi
ok "JWT token obtained"
info "Token: ${TOKEN:0:20}..."

# ─── Register flows for each HTTP method ─────────────────────────────────────
banner "2. Register Flows for GET, POST, PUT, DELETE"

# GET flow
curl -s -X POST $CP/flows -H "Content-Type: application/json" -d '{
  "id": "http-get-test",
  "name": "HTTP GET Test",
  "trigger": {"type": "http", "path": "/test", "method": "GET"},
  "steps": [
    {
      "type": "log",
      "name": "log_get",
      "message": "GET request received"
    }
  ]
}' > /dev/null
ok "Registered GET /test"

# POST flow
curl -s -X POST $CP/flows -H "Content-Type: application/json" -d '{
  "id": "http-post-test",
  "name": "HTTP POST Test",
  "trigger": {"type": "http", "path": "/test", "method": "POST"},
  "steps": [
    {
      "type": "log",
      "name": "log_post",
      "message": "POST request with body: {{trigger.body}}"
    }
  ]
}' > /dev/null
ok "Registered POST /test"

# PUT flow
curl -s -X POST $CP/flows -H "Content-Type: application/json" -d '{
  "id": "http-put-test",
  "name": "HTTP PUT Test",
  "trigger": {"type": "http", "path": "/test", "method": "PUT"},
  "steps": [
    {
      "type": "log",
      "name": "log_put",
      "message": "PUT request with data: {{trigger.body.data}}"
    }
  ]
}' > /dev/null
ok "Registered PUT /test"

# DELETE flow
curl -s -X POST $CP/flows -H "Content-Type: application/json" -d '{
  "id": "http-delete-test",
  "name": "HTTP DELETE Test",
  "trigger": {"type": "http", "path": "/test", "method": "DELETE"},
  "steps": [
    {
      "type": "log",
      "name": "log_delete",
      "message": "DELETE request for id: {{trigger.body.id}}"
    }
  ]
}' > /dev/null
ok "Registered DELETE /test"

sleep 2  # Wait for NATS propagation

# ─── Test GET ─────────────────────────────────────────────────────────────────
banner "3. Test GET Request"

code=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $TOKEN" \
  $DP/api/trigger/test)
assert_http "GET /api/trigger/test" "200" "$code"

# ─── Test POST ────────────────────────────────────────────────────────────────
banner "4. Test POST Request with Body"

code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice","email":"alice@example.com"}' \
  $DP/api/trigger/test)
assert_http "POST /api/trigger/test with body" "200" "$code"

# ─── Test PUT ─────────────────────────────────────────────────────────────────
banner "5. Test PUT Request with Body"

code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X PUT \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"id":1,"data":"updated value"}' \
  $DP/api/trigger/test)
assert_http "PUT /api/trigger/test with body" "200" "$code"

# ─── Test DELETE ──────────────────────────────────────────────────────────────
banner "6. Test DELETE Request with Body"

code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X DELETE \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"id":99}' \
  $DP/api/trigger/test)
assert_http "DELETE /api/trigger/test with body" "200" "$code"

# ─── Test method mismatch (404) ───────────────────────────────────────────────
banner "7. Test Method Mismatch (Expect 404)"

# Try POST on a GET-only endpoint
curl -s -X POST $CP/flows -H "Content-Type: application/json" -d '{
  "id": "http-get-only",
  "name": "GET Only",
  "trigger": {"type": "http", "path": "/getonly", "method": "GET"},
  "steps": [{"type": "log", "name": "l", "message": "GET only"}]
}' > /dev/null
sleep 1

code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST \
  -H "Authorization: Bearer $TOKEN" \
  $DP/api/trigger/getonly)
assert_http "POST on GET-only endpoint (expect 404)" "404" "$code"

# Verify GET works
code=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $TOKEN" \
  $DP/api/trigger/getonly)
assert_http "GET on GET-only endpoint (expect 200)" "200" "$code"

# ─── Test nonexistent path (404) ──────────────────────────────────────────────
banner "8. Test Nonexistent Path (Expect 404)"

code=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $TOKEN" \
  $DP/api/trigger/does-not-exist)
assert_http "GET on nonexistent path (expect 404)" "404" "$code"

# ─── Test same path, different methods ───────────────────────────────────────
banner "9. Test Multiple Methods on Same Path"

# We already have GET, POST, PUT, DELETE on /test
# Verify each works independently

info "Testing all methods on /test..."
for method in GET POST PUT DELETE; do
  code=$(curl -s -o /dev/null -w "%{http_code}" \
    -X $method \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"test":"data"}' \
    $DP/api/trigger/test 2>/dev/null)
  
  if [ "$code" == "200" ]; then
    ok "$method /test → 200"
  else
    fail "$method /test → $code (expected 200)"
  fi
done

# ─── Test unauthenticated requests (401) ──────────────────────────────────────
banner "10. Test Unauthenticated Requests (Expect 401)"

code=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"test":"data"}' \
  $DP/api/trigger/test)
assert_http "POST without auth (expect 401)" "401" "$code"

# ─── Summary ──────────────────────────────────────────────────────────────────
banner "✅ All HTTP Trigger Tests Passed"

echo ""
echo -e "${BOLD}HTTP Methods Verified:${NC}"
echo -e "  ${GREEN}✓${NC} GET — Read operations"
echo -e "  ${GREEN}✓${NC} POST — Create operations with body"
echo -e "  ${GREEN}✓${NC} PUT — Update operations with body"
echo -e "  ${GREEN}✓${NC} DELETE — Delete operations with body"
echo ""
echo -e "${BOLD}Features Tested:${NC}"
echo -e "  ${GREEN}✓${NC} Request body handling (POST/PUT/DELETE)"
echo -e "  ${GREEN}✓${NC} Path + method matching"
echo -e "  ${GREEN}✓${NC} Multiple methods on same path"
echo -e "  ${GREEN}✓${NC} 404 for method mismatch"
echo -e "  ${GREEN}✓${NC} 404 for nonexistent paths"
echo -e "  ${GREEN}✓${NC} 401 for unauthenticated requests"
