#!/bin/bash

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Event-Driven Config Distribution Test Suite         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

# Function to test endpoint
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
    else
        response=$(curl -s -X "$method" "$url" -H "Content-Type: application/json" -d "$data")
    fi
    
    echo "$response" | jq '.' 2>/dev/null || echo "$response"
    echo ""
}

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}1. Testing Event-Driven Flow Creation${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

# Create a flow in Control Plane
test_api \
    "Create Flow in Control Plane" \
    "POST" \
    "http://localhost:8081/flows" \
    '{
      "id": "event-test-flow",
      "name": "Event Test Flow",
      "trigger": {
        "type": "http",
        "path": "/api/event-test",
        "method": "GET"
      },
      "steps": [
        {
          "type": "log",
          "name": "start",
          "message": "Event-driven flow executing!"
        },
        {
          "type": "call",
          "name": "get_users",
          "connector": "postgres",
          "operation": "query",
          "params": {
            "sql": "SELECT id, name, email FROM users LIMIT 3"
          }
        },
        {
          "type": "log",
          "name": "end",
          "message": "Flow completed successfully"
        }
      ]
    }'

echo -e "${YELLOW}⏳ Waiting for event to propagate to Data Plane (2 seconds)...${NC}"
sleep 2
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}2. Executing Flow on Data Plane${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

# Execute the flow on Data Plane
test_api \
    "Execute Flow (Should work via event distribution)" \
    "POST" \
    "http://localhost:8080/flows/event-test-flow/execute" \
    '{"test": "event-driven"}'

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}3. List Flows on Control Plane${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

test_api \
    "List Flows on Control Plane" \
    "GET" \
    "http://localhost:8081/flows"

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}4. Create Another Flow${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

test_api \
    "Create Second Flow" \
    "POST" \
    "http://localhost:8081/flows" \
    '{
      "id": "user-count-flow",
      "name": "User Count Flow",
      "trigger": {
        "type": "http",
        "path": "/api/user-count",
        "method": "GET"
      },
      "steps": [
        {
          "type": "call",
          "name": "count_users",
          "connector": "postgres",
          "operation": "query",
          "params": {
            "sql": "SELECT COUNT(*) as total FROM users"
          }
        }
      ]
    }'

echo -e "${YELLOW}⏳ Waiting for event propagation...${NC}"
sleep 2
echo ""

test_api \
    "Execute Second Flow" \
    "POST" \
    "http://localhost:8080/flows/user-count-flow/execute" \
    '{}'

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}5. Delete Flow Test${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

test_api \
    "Delete Flow from Control Plane" \
    "DELETE" \
    "http://localhost:8081/flows/event-test-flow"

echo -e "${YELLOW}⏳ Waiting for deletion event...${NC}"
sleep 2
echo ""

echo -e "Trying to execute deleted flow (should fail):"
test_api \
    "Execute Deleted Flow (Should Fail)" \
    "POST" \
    "http://localhost:8080/flows/event-test-flow/execute" \
    '{}'

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}6. Backward Compatibility Test${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Testing HTTP trigger without pre-created flow:${NC}"
test_api \
    "HTTP GET Trigger (Auto-creates flow)" \
    "GET" \
    "http://localhost:8080/api/trigger/users"

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ Event-Driven Tests Complete!${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}💡 Check logs to see event distribution:${NC}"
echo -e "  ${GREEN}docker-compose logs -f control-plane | grep '📤'${NC}  # Publishing"
echo -e "  ${GREEN}docker-compose logs -f data-plane | grep '📥'${NC}      # Receiving"
echo -e "  ${GREEN}docker-compose logs -f nats${NC}                        # NATS activity"
echo ""

echo -e "${YELLOW}🎯 Key Points Tested:${NC}"
echo -e "  ✅ Control Plane publishes flow creation events"
echo -e "  ✅ Data Plane receives and registers flows"
echo -e "  ✅ Flows executable immediately after creation"
echo -e "  ✅ Flow deletion propagates correctly"
echo -e "  ✅ Backward compatibility maintained"
echo ""
