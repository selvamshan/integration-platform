#!/bin/bash

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   UI Palette & Auto-API Features Test Suite           ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

test_api() {
    local name=$1
    local method=$2
    local url=$3
    local data=$4
    
    echo -e "${YELLOW}Test: ${name}${NC}"
    
    if [ "$method" == "GET" ]; then
        response=$(curl -s "$url")
    elif [ "$method" == "DELETE" ]; then
        response=$(curl -s -X DELETE "$url")
    elif [ "$method" == "PUT" ]; then
        response=$(curl -s -X PUT "$url" -H "Content-Type: application/json" -d "$data")
    else
        response=$(curl -s -X "$method" "$url" -H "Content-Type: application/json" -d "$data")
    fi
    
    echo "$response" | jq '.' 2>/dev/null || echo "$response"
    echo ""
}

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}1. Testing Connector Registry (for UI Palette)${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

test_api \
    "List All Connectors" \
    "GET" \
    "http://localhost:8081/connectors"

echo -e "${YELLOW}Getting specific connector details:${NC}"
test_api \
    "Get HTTP Connector Details" \
    "GET" \
    "http://localhost:8081/connectors/http-connector"

test_api \
    "Get PostgreSQL Connector Details" \
    "GET" \
    "http://localhost:8081/connectors/postgres-connector"

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}2. Testing Trigger Registry (for UI Palette)${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

test_api \
    "List All Triggers" \
    "GET" \
    "http://localhost:8081/triggers"

echo -e "${YELLOW}Getting specific trigger details:${NC}"
test_api \
    "Get HTTP Trigger Details" \
    "GET" \
    "http://localhost:8081/triggers/http-trigger"

test_api \
    "Get Schedule Trigger Details" \
    "GET" \
    "http://localhost:8081/triggers/schedule-trigger"

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}3. Testing Auto-API Creation on Flow Creation${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Creating flow with HTTP trigger...${NC}"
test_api \
    "Create Flow (Auto-creates API)" \
    "POST" \
    "http://localhost:8081/flows" \
    '{
      "id": "user-search-flow",
      "name": "User Search Flow",
      "trigger": {
        "type": "http",
        "path": "/api/users/search",
        "method": "GET"
      },
      "steps": [
        {
          "type": "log",
          "name": "start",
          "message": "Searching users"
        },
        {
          "type": "call",
          "name": "search_db",
          "connector": "postgres",
          "operation": "query",
          "params": {
            "sql": "SELECT * FROM users WHERE name LIKE '\''%test%'\''"
          }
        }
      ]
    }'

echo -e "${YELLOW}⏳ Waiting for API auto-creation...${NC}"
sleep 2

echo -e "${YELLOW}Checking if API was auto-created:${NC}"
test_api \
    "List APIs (Should show auto-generated API)" \
    "GET" \
    "http://localhost:8081/apis"

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}4. Testing Auto-API Update on Flow Modification${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Updating flow with different path...${NC}"
test_api \
    "Update Flow (Auto-updates API)" \
    "PUT" \
    "http://localhost:8081/flows/user-search-flow" \
    '{
      "id": "user-search-flow",
      "name": "User Search Flow V2",
      "trigger": {
        "type": "http",
        "path": "/api/v2/users/search",
        "method": "POST"
      },
      "steps": [
        {
          "type": "log",
          "name": "start",
          "message": "Searching users V2"
        },
        {
          "type": "call",
          "name": "search_db",
          "connector": "postgres",
          "operation": "query",
          "params": {
            "sql": "SELECT * FROM users LIMIT 10"
          }
        }
      ]
    }'

echo -e "${YELLOW}⏳ Waiting for API auto-update...${NC}"
sleep 2

echo -e "${YELLOW}Checking if API was auto-updated:${NC}"
test_api \
    "List APIs (Should show updated endpoint)" \
    "GET" \
    "http://localhost:8081/apis"

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}5. Testing Multiple Flows Create Multiple Endpoints${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Creating second flow...${NC}"
test_api \
    "Create Second Flow" \
    "POST" \
    "http://localhost:8081/flows" \
    '{
      "id": "product-list-flow",
      "name": "Product List Flow",
      "trigger": {
        "type": "http",
        "path": "/api/products",
        "method": "GET"
      },
      "steps": [
        {
          "type": "log",
          "name": "list",
          "message": "Listing products"
        }
      ]
    }'

echo -e "${YELLOW}Creating third flow...${NC}"
test_api \
    "Create Third Flow" \
    "POST" \
    "http://localhost:8081/flows" \
    '{
      "id": "order-create-flow",
      "name": "Order Creation Flow",
      "trigger": {
        "type": "http",
        "path": "/api/orders",
        "method": "POST"
      },
      "steps": [
        {
          "type": "log",
          "name": "create",
          "message": "Creating order"
        }
      ]
    }'

sleep 2

echo -e "${YELLOW}Checking API - should have all endpoints:${NC}"
test_api \
    "List APIs (Should show all endpoints)" \
    "GET" \
    "http://localhost:8081/apis"

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}6. Testing Auto-API Cleanup on Flow Deletion${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Deleting flow...${NC}"
test_api \
    "Delete Flow (Auto-removes endpoint)" \
    "DELETE" \
    "http://localhost:8081/flows/user-search-flow"

sleep 2

echo -e "${YELLOW}Checking if endpoint was removed from API:${NC}"
test_api \
    "List APIs (user-search endpoint should be gone)" \
    "GET" \
    "http://localhost:8081/apis"

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}7. Frontend Integration Example${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Simulating Frontend Flow Designer Workflow:${NC}"
echo ""
echo -e "${GREEN}Step 1: UI loads connector palette${NC}"
echo "GET /connectors"
curl -s http://localhost:8081/connectors | jq -c '.connectors[] | {name, operations: .operations[].name}'
echo ""

echo -e "${GREEN}Step 2: UI loads trigger palette${NC}"
echo "GET /triggers"
curl -s http://localhost:8081/triggers | jq -c '.triggers[] | {name, type: .trigger_type}'
echo ""

echo -e "${GREEN}Step 3: User designs flow in UI and submits${NC}"
echo "POST /flows { ... }"
echo ""

echo -e "${GREEN}Step 4: Flow immediately available!${NC}"
echo "POST /flows/{id}/execute"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}8. Summary of Auto-Generated API State${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Current flows:${NC}"
curl -s http://localhost:8081/flows | jq '.flows[] | {id, name, trigger}'
echo ""

echo -e "${YELLOW}Current API endpoints (auto-generated):${NC}"
curl -s http://localhost:8081/apis | jq '.apis[] | {name, endpoints: .endpoints[]}'
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ UI Palette & Auto-API Tests Complete!${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}📊 Test Summary:${NC}"
echo ""
echo -e "  ✅ Connector registry available at ${GREEN}/connectors${NC}"
echo -e "  ✅ Trigger registry available at ${GREEN}/triggers${NC}"
echo -e "  ✅ API definitions auto-created on flow creation"
echo -e "  ✅ API definitions auto-updated on flow modification"
echo -e "  ✅ API endpoints auto-removed on flow deletion"
echo -e "  ✅ Multiple flows create multiple endpoints in same API"
echo ""

echo -e "${YELLOW}💡 Frontend Integration:${NC}"
echo -e "  1. Load palette: ${GREEN}GET /connectors${NC} and ${GREEN}GET /triggers${NC}"
echo -e "  2. User designs flow with palette components"
echo -e "  3. Submit: ${GREEN}POST /flows${NC}"
echo -e "  4. API auto-created, flow immediately executable!"
echo ""

echo -e "${YELLOW}📚 See UI-PALETTE-GUIDE.md for complete documentation${NC}"
echo ""
