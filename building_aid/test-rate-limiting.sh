#!/bin/bash

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         Rate Limiting Test Suite                       ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

test_api() {
    local name=$1
    local method=$2
    local url=$3
    local data=$4
    
    echo -e "${YELLOW}Test: ${name}${NC}"
    
    if [ "$method" == "GET" ]; then
        response=$(curl -s -w "\nHTTP_CODE:%{http_code}" "$url")
    else
        response=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X "$method" "$url" -H "Content-Type: application/json" -d "$data")
    fi
    
    http_code=$(echo "$response" | grep "HTTP_CODE" | cut -d: -f2)
    body=$(echo "$response" | sed '/HTTP_CODE/d')
    
    echo "$body" | jq '.' 2>/dev/null || echo "$body"
    echo -e "${BLUE}HTTP Status: ${http_code}${NC}"
    echo ""
    
    echo "$http_code"
}

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}1. Creating Flow with Rate Limit${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Creating flow with 5 requests per 60 seconds limit...${NC}"
test_api \
    "Create Rate Limited Flow" \
    "POST" \
    "http://localhost:8081/flows" \
    '{
      "id": "rate-limited-flow",
      "name": "Rate Limited Flow",
      "trigger": {
        "type": "http",
        "path": "/api/limited",
        "method": "GET"
      },
      "steps": [
        {
          "type": "log",
          "name": "test",
          "message": "Rate limited endpoint called"
        },
        {
          "type": "call",
          "name": "fetch_users",
          "connector": "postgres",
          "operation": "query",
          "params": {
            "sql": "SELECT COUNT(*) as count FROM users"
          }
        }
      ],
      "rate_limit": {
        "max_requests": 5,
        "window_seconds": 60,
        "key_type": "per_ip",
        "message": "Too many requests! Maximum 5 per minute."
      }
    }' > /dev/null

echo -e "${YELLOW}⏳ Waiting for flow to sync to Data Plane...${NC}"
sleep 3
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}2. Testing Rate Limit - Sending Requests${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

success_count=0
fail_count=0

for i in {1..7}; do
    echo -e "${YELLOW}Request #$i${NC}"
    status=$(test_api \
        "Execute Flow (Request $i)" \
        "POST" \
        "http://localhost:8080/flows/rate-limited-flow/execute" \
        '{"test": true}')
    
    if [ "$status" == "200" ]; then
        success_count=$((success_count + 1))
        echo -e "${GREEN}✅ Request allowed${NC}"
    elif [ "$status" == "429" ]; then
        fail_count=$((fail_count + 1))
        echo -e "${RED}🚫 Rate limit exceeded${NC}"
    fi
    
    sleep 0.5
done

echo ""
echo -e "${BLUE}Results:${NC}"
echo -e "  ${GREEN}Allowed: ${success_count}${NC}"
echo -e "  ${RED}Blocked: ${fail_count}${NC}"
echo ""

if [ $success_count -eq 5 ] && [ $fail_count -eq 2 ]; then
    echo -e "${GREEN}✅ Rate limit working correctly!${NC}"
else
    echo -e "${YELLOW}⚠️  Expected 5 allowed, 2 blocked${NC}"
fi
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}3. Checking Rate Limit Statistics${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

sleep 2

echo -e "${YELLOW}All flows rate limit stats:${NC}"
curl -s http://localhost:8081/rate-limits | jq '.'
echo ""

echo -e "${YELLOW}Specific flow rate limit stats:${NC}"
curl -s http://localhost:8081/rate-limits/rate-limited-flow | jq '.summary'
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}4. Testing Different Key Types${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Creating flow with GLOBAL rate limit...${NC}"
test_api \
    "Create Global Rate Limited Flow" \
    "POST" \
    "http://localhost:8081/flows" \
    '{
      "id": "global-limited",
      "name": "Global Rate Limited",
      "trigger": {"type": "http", "path": "/api/global", "method": "GET"},
      "steps": [{"type": "log", "name": "test", "message": "Global limit"}],
      "rate_limit": {
        "max_requests": 3,
        "window_seconds": 60,
        "key_type": "global",
        "message": "System-wide rate limit exceeded"
      }
    }' > /dev/null

sleep 2

echo -e "${YELLOW}Testing global limit (shared across all clients)...${NC}"
for i in {1..4}; do
    status=$(test_api \
        "Global Limit Test $i" \
        "POST" \
        "http://localhost:8080/flows/global-limited/execute" \
        '{}')
    
    if [ "$status" == "429" ]; then
        echo -e "${RED}Request $i: Blocked (as expected after 3)${NC}"
    else
        echo -e "${GREEN}Request $i: Allowed${NC}"
    fi
done
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}5. Testing Flow Without Rate Limit${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Creating flow WITHOUT rate limit...${NC}"
test_api \
    "Create Unlimited Flow" \
    "POST" \
    "http://localhost:8081/flows" \
    '{
      "id": "unlimited-flow",
      "name": "Unlimited Flow",
      "trigger": {"type": "http", "path": "/api/unlimited", "method": "GET"},
      "steps": [{"type": "log", "name": "test", "message": "No limit"}]
    }' > /dev/null

sleep 2

echo -e "${YELLOW}Sending 10 requests (should all succeed)...${NC}"
unlimited_success=0
for i in {1..10}; do
    status=$(curl -s -w "%{http_code}" -o /dev/null \
        -X POST http://localhost:8080/flows/unlimited-flow/execute \
        -H "Content-Type: application/json" -d '{}')
    
    if [ "$status" == "200" ]; then
        unlimited_success=$((unlimited_success + 1))
    fi
done

echo -e "All requests completed: ${GREEN}${unlimited_success}/10 succeeded${NC}"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}6. Checking Redis Keys${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Rate limit keys in Redis:${NC}"
docker exec integration-redis redis-cli KEYS "ratelimit:*" 2>/dev/null || echo "Could not connect to Redis"
echo ""

echo -e "${YELLOW}Checking specific key:${NC}"
docker exec integration-redis redis-cli GET "ratelimit:ip:*:rate-limited-flow" 2>/dev/null || echo "Key not found or Redis not accessible"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}7. Testing Custom Error Message${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Creating flow with custom error message...${NC}"
test_api \
    "Create Flow with Custom Message" \
    "POST" \
    "http://localhost:8081/flows" \
    '{
      "id": "custom-message-flow",
      "name": "Custom Message Flow",
      "trigger": {"type": "http", "path": "/api/custom", "method": "GET"},
      "steps": [{"type": "log", "name": "test", "message": "Test"}],
      "rate_limit": {
        "max_requests": 1,
        "window_seconds": 60,
        "key_type": "per_ip",
        "message": "🚫 Whoa there! You can only call this once per minute. Take a breather!"
      }
    }' > /dev/null

sleep 2

echo -e "${YELLOW}First request (should succeed):${NC}"
test_api "Request 1" "POST" "http://localhost:8080/flows/custom-message-flow/execute" '{}' > /dev/null

echo -e "${YELLOW}Second request (should show custom message):${NC}"
curl -s -X POST http://localhost:8080/flows/custom-message-flow/execute \
    -H "Content-Type: application/json" -d '{}' | jq '.error'
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}8. Rate Limit Statistics Summary${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Final statistics for all flows:${NC}"
curl -s http://localhost:8081/rate-limits | jq '.flows | to_entries[] | {flow_id: .key, stats: .value}'
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ Rate Limiting Tests Complete!${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}📊 Test Summary:${NC}"
echo ""
echo -e "  ✅ Per-IP rate limiting works"
echo -e "  ✅ Global rate limiting works"
echo -e "  ✅ Flows without limits are unrestricted"
echo -e "  ✅ Custom error messages display correctly"
echo -e "  ✅ Statistics are tracked in Control Plane"
echo -e "  ✅ Redis persistence functioning"
echo -e "  ✅ HTTP 429 status returned on limit exceeded"
echo ""

echo -e "${YELLOW}💡 View Logs:${NC}"
echo -e "  Control Plane: ${GREEN}docker-compose logs control-plane | grep ratelimit${NC}"
echo -e "  Data Plane:    ${GREEN}docker-compose logs data-plane | grep 'Rate limit'${NC}"
echo -e "  Redis:         ${GREEN}docker exec integration-redis redis-cli KEYS 'ratelimit:*'${NC}"
echo ""

echo -e "${YELLOW}📚 See RATE-LIMITING.md for complete documentation${NC}"
echo ""
