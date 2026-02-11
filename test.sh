#!/bin/bash

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Integration Platform - Test Suite                 ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

# Wait for services to be ready
echo -e "${YELLOW}⏳ Waiting for services to be ready...${NC}"
sleep 5

# Function to make request and display result
test_endpoint() {
    local name=$1
    local method=$2
    local url=$3
    local data=$4
    
    echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}Test: ${name}${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    if [ "$method" == "GET" ]; then
        echo -e "Request: ${GREEN}$method $url${NC}"
        response=$(curl -s "$url")
    else
        echo -e "Request: ${GREEN}$method $url${NC}"
        echo -e "Body: $data"
        response=$(curl -s -X "$method" "$url" -H "Content-Type: application/json" -d "$data")
    fi
    
    echo -e "\nResponse:"
    echo "$response" | jq '.' 2>/dev/null || echo "$response"
}

# Test 1: Control Plane Health
test_endpoint \
    "Control Plane Health Check" \
    "GET" \
    "http://localhost:8081/health"

# Test 2: Data Plane Health
test_endpoint \
    "Data Plane Health Check" \
    "GET" \
    "http://localhost:8080/health"

# Test 3: HTTP Trigger - Simple GET
test_endpoint \
    "HTTP Trigger - Get Users" \
    "GET" \
    "http://localhost:8080/api/trigger/users"

# Test 4: HTTP Trigger - Another endpoint
test_endpoint \
    "HTTP Trigger - Get Data" \
    "GET" \
    "http://localhost:8080/api/trigger/data"

# Test 5: Execute Flow with Custom Payload
test_endpoint \
    "Execute Flow - Database Query" \
    "POST" \
    "http://localhost:8080/flows/test-flow/execute" \
    '{"message": "Testing database connector", "limit": 5}'

# Test 6: List APIs
test_endpoint \
    "List APIs" \
    "GET" \
    "http://localhost:8081/apis"

# Test 7: List Flows
test_endpoint \
    "List Flows" \
    "GET" \
    "http://localhost:8081/flows"

echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ All tests completed!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}💡 Tips:${NC}"
echo -e "  • View logs: ${GREEN}docker-compose logs -f${NC}"
echo -e "  • View data-plane logs: ${GREEN}docker-compose logs -f data-plane${NC}"
echo -e "  • Stop services: ${GREEN}docker-compose down${NC}"
echo ""
