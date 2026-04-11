#!/bin/bash
set -e
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'

CP="http://localhost:8081"

banner() { echo -e "\n${BLUE}${BOLD}══════════════════════════════════════════${NC}"; echo -e "${BLUE}${BOLD}  $1${NC}"; echo -e "${BLUE}${BOLD}══════════════════════════════════════════${NC}"; }
ok()     { echo -e "${GREEN}✅ $1${NC}"; }
fail()   { echo -e "${RED}❌ $1${NC}"; exit 1; }
info()   { echo -e "${YELLOW}   $1${NC}"; }

banner "RBAC Endpoint Test"

echo ""
echo "This test verifies that the user management endpoints are:"
echo "1. Registered in the router"
echo "2. Accessible (even without Keycloak authentication)"
echo ""
info "Note: With RBAC middleware disabled, endpoints should return"
info "Internal Server Error (no user context) or work without auth."
echo ""

# ─── Test endpoint existence ──────────────────────────────────────────────────
banner "1. Test Endpoint Registration"

# POST /users/invite
code=$(curl -s -o /dev/null -w "%{http_code}" -X POST $CP/users/invite \
  -H "Content-Type: application/json" -d '{"email":"test@example.com","role":"developer"}' 2>/dev/null)

if [ "$code" == "404" ]; then
  fail "POST /users/invite returns 404 (endpoint not registered)"
else
  ok "POST /users/invite is registered (status: $code)"
fi

# GET /users
code=$(curl -s -o /dev/null -w "%{http_code}" $CP/users 2>/dev/null)

if [ "$code" == "404" ]; then
  fail "GET /users returns 404 (endpoint not registered)"
else
  ok "GET /users is registered (status: $code)"
fi

# GET /users/me
code=$(curl -s -o /dev/null -w "%{http_code}" $CP/users/me 2>/dev/null)

if [ "$code" == "404" ]; then
  fail "GET /users/me returns 404 (endpoint not registered)"
else
  ok "GET /users/me is registered (status: $code)"
fi

# DELETE /users/:id
code=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE $CP/users/test-user-id 2>/dev/null)

if [ "$code" == "404" ]; then
  fail "DELETE /users/:user_id returns 404 (endpoint not registered)"
else
  ok "DELETE /users/:user_id is registered (status: $code)"
fi

# ─── Expected behavior without RBAC ───────────────────────────────────────────
banner "2. Expected Behavior (RBAC Disabled)"

info "With RBAC middleware commented out, endpoints will:"
info "• Return 500 (no user context from middleware)"
info "• Or return Keycloak errors (if Keycloak configured)"
echo ""

RESP=$(curl -s $CP/users/me 2>/dev/null)
echo "GET /users/me response:"
echo "$RESP" | jq '.' 2>/dev/null || echo "$RESP"

# ─── Test with RBAC enabled (if Keycloak configured) ──────────────────────────
banner "3. Keycloak Integration Check"

if [ -n "$KEYCLOAK_CLIENT_SECRET" ]; then
  info "KEYCLOAK_CLIENT_SECRET is set"
  
  # Try to get a token
  TOKEN_RESP=$(curl -s -X POST http://localhost:8180/auth/realms/integration-platform/protocol/openid-connect/token \
    -d "client_id=control-plane" \
    -d "client_secret=$KEYCLOAK_CLIENT_SECRET" \
    -d "username=admin-user" \
    -d "password=admin123" \
    -d "grant_type=password" 2>/dev/null || echo '{}')
  
  TOKEN=$(echo "$TOKEN_RESP" | jq -r '.access_token // empty')
  
  if [ -n "$TOKEN" ] && [ "$TOKEN" != "null" ]; then
    ok "Successfully obtained Keycloak token"
    info "Token: ${TOKEN:0:30}..."
    
    # Test authenticated request
    echo ""
    info "Testing GET /users/me with token..."
    AUTHED_RESP=$(curl -s $CP/users/me -H "Authorization: Bearer $TOKEN" 2>/dev/null)
    echo "$AUTHED_RESP" | jq '.' 2>/dev/null || echo "$AUTHED_RESP"
  else
    info "Could not obtain token (Keycloak might not be configured)"
    info "This is expected if you haven't set up Keycloak yet"
  fi
else
  info "KEYCLOAK_CLIENT_SECRET not set"
  info "To test full RBAC, set up Keycloak and set this env var"
fi

# ─── Enable RBAC instructions ─────────────────────────────────────────────────
banner "4. Enabling RBAC"

echo "To enable RBAC authentication:"
echo ""
echo "1. Set up Keycloak (see RBAC-KEYCLOAK.md)"
echo "2. Uncomment RBAC middleware in control-plane/src/main.rs:"
echo ""
echo "   // .layer(middleware::from_fn(permission_middleware))"
echo "   // .layer(middleware::from_fn_with_state(keycloak.clone(), rbac_middleware))"
echo ""
echo "   Remove the // to enable"
echo ""
echo "3. Rebuild: docker-compose build control-plane"
echo "4. Restart: docker-compose restart control-plane"
echo ""

# ─── Summary ──────────────────────────────────────────────────────────────────
banner "✅ Endpoint Registration Verified"

echo ""
echo -e "${BOLD}All user management endpoints are registered:${NC}"
echo -e "  ${GREEN}✓${NC} POST /users/invite"
echo -e "  ${GREEN}✓${NC} GET /users"
echo -e "  ${GREEN}✓${NC} GET /users/me"
echo -e "  ${GREEN}✓${NC} DELETE /users/:user_id"
echo ""
echo -e "${BOLD}Status:${NC}"
echo -e "  • Endpoints: ${GREEN}Registered${NC}"
echo -e "  • RBAC Middleware: ${YELLOW}Disabled by default${NC}"
echo -e "  • To enable: See RBAC-KEYCLOAK.md"
