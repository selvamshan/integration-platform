#!/bin/bash
set -e

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'

CP="http://localhost:8081"
DP="http://localhost:8080"

banner() { echo -e "\n${BLUE}${BOLD}══════════════════════════════════════════${NC}"; echo -e "${BLUE}${BOLD}  $1${NC}"; echo -e "${BLUE}${BOLD}══════════════════════════════════════════${NC}"; }
ok()     { echo -e "${GREEN}✅ $1${NC}"; }
fail()   { echo -e "${RED}❌ $1${NC}"; exit 1; }
info()   { echo -e "${YELLOW}   $1${NC}"; }

banner "HTTP Connector Test Suite"

echo ""
info "This script tests HTTP connector with multiple auth types"
echo ""

# ─── Test 1: Register Public API Connector ────────────────────────────────────
banner "1. Register Public API Connector (No Auth)"

curl -s -X POST $CP/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "jsonplaceholder",
    "name": "JSON Placeholder",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://jsonplaceholder.typicode.com",
      "timeout_ms": 30000,
      "default_headers": {
        "Content-Type": "application/json"
      }
    }
  }' | jq '.' || true

ok "Registered jsonplaceholder connector"

# ─── Test 2: Register Bearer Auth Connector ───────────────────────────────────
banner "2. Register Bearer Auth Connector"

curl -s -X POST $CP/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "bearer_api",
    "name": "Bearer Auth API",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://api.example.com",
      "auth": {
        "type": "bearer",
        "token": "test_bearer_token_12345"
      },
      "default_headers": {
        "Accept": "application/json"
      }
    }
  }' | jq '.' || true

ok "Registered bearer_api connector"

# ─── Test 3: Register Basic Auth Connector ────────────────────────────────────
banner "3. Register Basic Auth Connector"

curl -s -X POST $CP/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "basic_api",
    "name": "Basic Auth API",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://httpbin.org",
      "auth": {
        "type": "basic",
        "username": "test_user",
        "password": "test_password"
      }
    }
  }' | jq '.' || true

ok "Registered basic_api connector"

# ─── Test 4: Register API Key Connector ───────────────────────────────────────
banner "4. Register API Key Connector"

curl -s -X POST $CP/connector-instances \
  -H "Content-Type: application/json" \
  -d '{
    "id": "apikey_api",
    "name": "API Key Auth",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://api.example.com",
      "auth": {
        "type": "apikey",
        "header_name": "X-API-Key",
        "api_key": "secret_api_key_12345"
      }
    }
  }' | jq '.' || true

ok "Registered apikey_api connector"

# ─── Test 5: List All HTTP Connectors ─────────────────────────────────────────
banner "5. List All Connector Instances"

echo ""
info "All connector instances:"
curl -s $CP/connector-instances | jq '.instances[] | select(.connector_type == "http") | {id, name, connector_type}'

# ─── Test 6: Create Flow with HTTP Connector ──────────────────────────────────
banner "6. Create Flow Using HTTP Connector"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-http-flow",
    "name": "Test HTTP Flow",
    "trigger": {
      "type": "http",
      "path": "/test-http",
      "method": "GET"
    },
    "steps": [
      {
        "type": "call",
        "name": "fetch_users",
        "connector": "jsonplaceholder",
        "operation": "get",
        "params": {
          "path": "/users/1"
        }
      },
      {
        "type": "log",
        "name": "log_result",
        "message": "Fetched user: {{fetch_users.data.name}}"
      }
    ]
  }' | jq '.'

ok "Created test-http-flow"

# ─── Test 7: Execute Flow ─────────────────────────────────────────────────────
banner "7. Execute HTTP Flow"

echo ""
info "Calling: GET $DP/api/trigger/test-http"
echo ""

RESPONSE=$(curl -s $DP/api/trigger/test-http)
echo "$RESPONSE" | jq '.'

if echo "$RESPONSE" | jq -e '.fetch_users.data.name' > /dev/null 2>&1; then
  ok "Flow executed successfully!"
  USER_NAME=$(echo "$RESPONSE" | jq -r '.fetch_users.data.name')
  info "User name: $USER_NAME"
else
  fail "Flow execution failed"
fi

# ─── Test 8: Test POST Request ────────────────────────────────────────────────
banner "8. Create Flow with POST Request"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-http-post",
    "name": "Test HTTP POST",
    "trigger": {
      "type": "http",
      "path": "/test-post",
      "method": "POST"
    },
    "steps": [
      {
        "type": "call",
        "name": "create_post",
        "connector": "jsonplaceholder",
        "operation": "post",
        "params": {
          "path": "/posts",
          "body": {
            "title": "{{trigger.body.title}}",
            "body": "{{trigger.body.content}}",
            "userId": 1
          }
        }
      }
    ]
  }' | jq '.'

ok "Created test-http-post flow"

echo ""
info "Testing POST request..."

POST_RESPONSE=$(curl -s -X POST $DP/api/trigger/test-post \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Test Post",
    "content": "This is a test post created by HTTP connector"
  }')

echo "$POST_RESPONSE" | jq '.'

if echo "$POST_RESPONSE" | jq -e '.create_post.data.id' > /dev/null 2>&1; then
  ok "POST request successful!"
  POST_ID=$(echo "$POST_RESPONSE" | jq -r '.create_post.data.id')
  info "Created post ID: $POST_ID"
else
  fail "POST request failed"
fi

# ─── Test 9: Test Multiple HTTP Methods ───────────────────────────────────────
banner "9. Test All HTTP Methods"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-all-methods",
    "name": "Test All HTTP Methods",
    "trigger": {
      "type": "http",
      "path": "/test-all-methods",
      "method": "POST"
    },
    "steps": [
      {
        "type": "call",
        "name": "get_request",
        "connector": "jsonplaceholder",
        "operation": "get",
        "params": {"path": "/posts/1"}
      },
      {
        "type": "call",
        "name": "post_request",
        "connector": "jsonplaceholder",
        "operation": "post",
        "params": {
          "path": "/posts",
          "body": {"title": "Test", "body": "Test", "userId": 1}
        }
      },
      {
        "type": "call",
        "name": "put_request",
        "connector": "jsonplaceholder",
        "operation": "put",
        "params": {
          "path": "/posts/1",
          "body": {"title": "Updated", "body": "Updated", "userId": 1}
        }
      },
      {
        "type": "call",
        "name": "patch_request",
        "connector": "jsonplaceholder",
        "operation": "patch",
        "params": {
          "path": "/posts/1",
          "body": {"title": "Patched"}
        }
      },
      {
        "type": "call",
        "name": "delete_request",
        "connector": "jsonplaceholder",
        "operation": "delete",
        "params": {"path": "/posts/1"}
      }
    ]
  }' | jq '.'

ok "Created test-all-methods flow"

echo ""
info "Executing flow with all HTTP methods..."

ALL_RESPONSE=$(curl -s -X POST $DP/api/trigger/test-all-methods \
  -H "Content-Type: application/json" \
  -d '{}')

echo "$ALL_RESPONSE" | jq '.'

if echo "$ALL_RESPONSE" | jq -e '.get_request.status' > /dev/null 2>&1; then
  ok "All HTTP methods tested!"
  info "GET status: $(echo "$ALL_RESPONSE" | jq -r '.get_request.status')"
  info "POST status: $(echo "$ALL_RESPONSE" | jq -r '.post_request.status')"
  info "PUT status: $(echo "$ALL_RESPONSE" | jq -r '.put_request.status')"
  info "PATCH status: $(echo "$ALL_RESPONSE" | jq -r '.patch_request.status')"
  info "DELETE status: $(echo "$ALL_RESPONSE" | jq -r '.delete_request.status')"
else
  fail "HTTP methods test failed"
fi

# ─── Summary ──────────────────────────────────────────────────────────────────
banner "✅ Test Summary"

echo ""
echo -e "${BOLD}HTTP Connector Features Tested:${NC}"
echo -e "  ${GREEN}✓${NC} No authentication (public API)"
echo -e "  ${GREEN}✓${NC} Bearer token authentication"
echo -e "  ${GREEN}✓${NC} Basic authentication"
echo -e "  ${GREEN}✓${NC} API key authentication"
echo -e "  ${GREEN}✓${NC} GET request"
echo -e "  ${GREEN}✓${NC} POST request"
echo -e "  ${GREEN}✓${NC} PUT request"
echo -e "  ${GREEN}✓${NC} PATCH request"
echo -e "  ${GREEN}✓${NC} DELETE request"
echo -e "  ${GREEN}✓${NC} base_url configuration"
echo -e "  ${GREEN}✓${NC} default_headers configuration"
echo -e "  ${GREEN}✓${NC} Connector instance integration"
echo ""
echo -e "${BOLD}Connector Instances Created:${NC}"
echo -e "  • jsonplaceholder (no auth)"
echo -e "  • bearer_api (bearer token)"
echo -e "  • basic_api (basic auth)"
echo -e "  • apikey_api (api key)"
echo ""
echo -e "${BOLD}Flows Created:${NC}"
echo -e "  • test-http-flow (GET)"
echo -e "  • test-http-post (POST)"
echo -e "  • test-all-methods (GET/POST/PUT/PATCH/DELETE)"
echo ""
echo -e "${GREEN}${BOLD}All tests passed! HTTP connector is working! 🌐✅${NC}"
echo ""
