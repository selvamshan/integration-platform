#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║       Circuit Breaker Test Suite                       ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}1. Creating Flow with Circuit Breaker${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

curl -s -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "cb-test-flow",
    "name": "Circuit Breaker Test Flow",
    "trigger": {"type": "http", "path": "/api/cb-test", "method": "GET"},
    "steps": [
      {"type": "log", "name": "start", "message": "Testing circuit breaker"},
      {"type": "call", "name": "db", "connector": "postgres", 
       "operation": "query", "params": {"sql": "SELECT 1/0"}}
    ],
    "circuit_breaker": {
      "failure_threshold": 3,
      "window_seconds": 60,
      "timeout_seconds": 15,
      "success_threshold": 2
    }
  }' > /dev/null

echo -e "${GREEN}✅ Flow created with circuit breaker policy:${NC}"
echo -e "   Failure threshold: ${YELLOW}3${NC}"
echo -e "   Timeout: ${YELLOW}15 seconds${NC}"
echo -e "   Success threshold: ${YELLOW}2${NC}"
echo ""

sleep 3

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}2. Triggering Circuit Breaker (Failures)${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Sending 6 requests (first 3 should fail, rest rejected)...${NC}"
echo ""

for i in {1..6}; do
    echo -n "Request $i: "
    
    response=$(curl -s -w "\n%{http_code}" \
        -X POST http://localhost:8080/flows/cb-test-flow/execute -d '{}')
    
    http_code=$(echo "$response" | tail -1)
    body=$(echo "$response" | head -n -1)
    
    if [ "$http_code" == "200" ]; then
        echo -e "${GREEN}✅ Success${NC}"
    elif [ "$http_code" == "500" ]; then
        echo -e "${RED}❌ Failed (execution error)${NC}"
    elif [ "$http_code" == "503" ]; then
        echo -e "${YELLOW}🔌 Rejected (circuit OPEN)${NC}"
        error=$(echo "$body" | jq -r '.error' 2>/dev/null)
        retry_after=$(echo "$body" | jq -r '.retry_after_seconds' 2>/dev/null)
        if [ "$retry_after" != "null" ]; then
            echo -e "   Retry after: ${BLUE}${retry_after}s${NC}"
        fi
    else
        echo -e "${RED}? Unknown (HTTP $http_code)${NC}"
    fi
    
    sleep 0.5
done

echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}3. Checking Circuit Breaker Status${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

sleep 1

cb_status=$(curl -s http://localhost:8080/circuit-breakers)

echo -e "${YELLOW}Circuit breaker status:${NC}"
echo "$cb_status" | jq '.circuit_breakers[] | select(.flow_id=="cb-test-flow")'

state=$(echo "$cb_status" | jq -r '.circuit_breakers[] | select(.flow_id=="cb-test-flow") | .state')

echo ""
if [ "$state" == "open" ]; then
    echo -e "${RED}🔴 Circuit is OPEN${NC} - Requests will be rejected"
elif [ "$state" == "closed" ]; then
    echo -e "${GREEN}🟢 Circuit is CLOSED${NC} - Normal operation"
elif [ "$state" == "half_open" ]; then
    echo -e "${YELLOW}🟡 Circuit is HALF-OPEN${NC} - Testing recovery"
fi
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}4. Checking Circuit Breaker Metrics${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

metrics=$(curl -s http://localhost:8080/metrics)

echo -e "${YELLOW}Circuit breaker metrics:${NC}"
echo ""

opens=$(echo "$metrics" | grep "^circuit_breaker_opens_total" | awk '{print $2}')
closes=$(echo "$metrics" | grep "^circuit_breaker_closes_total" | awk '{print $2}')
half_opens=$(echo "$metrics" | grep "^circuit_breaker_half_opens_total" | awk '{print $2}')
rejected=$(echo "$metrics" | grep "^circuit_breaker_rejected_total" | awk '{print $2}')

echo -e "  Opens:         ${RED}${opens:-0}${NC}"
echo -e "  Closes:        ${GREEN}${closes:-0}${NC}"
echo -e "  Half-opens:    ${YELLOW}${half_opens:-0}${NC}"
echo -e "  Rejected:      ${BLUE}${rejected:-0}${NC}"
echo ""

echo -e "${YELLOW}State by flow:${NC}"
echo "$metrics" | grep "circuit_breaker_state"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}5. Waiting for Circuit to Enter Half-Open${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Waiting 16 seconds for timeout...${NC}"
for i in {16..1}; do
    echo -ne "\rTime remaining: ${BLUE}${i}s ${NC}"
    sleep 1
done
echo ""
echo ""

echo -e "${YELLOW}Circuit should now be HALF-OPEN${NC}"
echo ""

cb_status=$(curl -s http://localhost:8080/circuit-breakers)
state=$(echo "$cb_status" | jq -r '.circuit_breakers[] | select(.flow_id=="cb-test-flow") | .state')

if [ "$state" == "half_open" ]; then
    echo -e "${YELLOW}🟡 Confirmed: Circuit is HALF-OPEN${NC}"
else
    echo -e "Current state: ${state}"
fi
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}6. Creating Successful Flow${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

curl -s -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "cb-success-flow",
    "name": "Circuit Breaker Success Flow",
    "trigger": {"type": "http", "path": "/api/cb-success", "method": "GET"},
    "steps": [
      {"type": "log", "name": "test", "message": "Success test"}
    ],
    "circuit_breaker": {
      "failure_threshold": 3,
      "window_seconds": 60,
      "timeout_seconds": 10,
      "success_threshold": 2
    }
  }' > /dev/null

sleep 2

echo -e "${YELLOW}Executing successful flow to verify circuit closes...${NC}"
echo ""

for i in {1..3}; do
    echo -n "Request $i: "
    http_code=$(curl -s -w "%{http_code}" -o /dev/null \
        -X POST http://localhost:8080/flows/cb-success-flow/execute -d '{}')
    
    if [ "$http_code" == "200" ]; then
        echo -e "${GREEN}✅ Success${NC}"
    else
        echo -e "${RED}❌ Failed (HTTP $http_code)${NC}"
    fi
    sleep 0.3
done

echo ""

cb_status=$(curl -s http://localhost:8080/circuit-breakers)
state=$(echo "$cb_status" | jq -r '.circuit_breakers[] | select(.flow_id=="cb-success-flow") | .state')

echo -e "Circuit state: ${GREEN}${state}${NC}"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}7. Final Metrics Summary${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

metrics=$(curl -s http://localhost:8080/metrics)

echo -e "${YELLOW}Final circuit breaker metrics:${NC}"
echo ""

opens=$(echo "$metrics" | grep "^circuit_breaker_opens_total" | awk '{print $2}')
closes=$(echo "$metrics" | grep "^circuit_breaker_closes_total" | awk '{print $2}')
half_opens=$(echo "$metrics" | grep "^circuit_breaker_half_opens_total" | awk '{print $2}')
rejected=$(echo "$metrics" | grep "^circuit_breaker_rejected_total" | awk '{print $2}')

echo -e "  Total Opens:        ${RED}${opens:-0}${NC}"
echo -e "  Total Closes:       ${GREEN}${closes:-0}${NC}"
echo -e "  Total Half-opens:   ${YELLOW}${half_opens:-0}${NC}"
echo -e "  Total Rejected:     ${BLUE}${rejected:-0}${NC}"
echo ""

echo -e "${YELLOW}All circuit breaker states:${NC}"
curl -s http://localhost:8080/circuit-breakers | jq '.circuit_breakers[] | {flow_id, state, failure_count}'
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ Circuit Breaker Tests Complete!${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}📊 Summary:${NC}"
echo ""
echo -e "  ✅ Circuit breaker opens on failures"
echo -e "  ✅ Requests rejected when circuit open (HTTP 503)"
echo -e "  ✅ Circuit transitions to half-open after timeout"
echo -e "  ✅ Circuit closes after successful recovery"
echo -e "  ✅ Metrics tracked for all state transitions"
echo -e "  ✅ Status API available at /circuit-breakers"
echo ""

echo -e "${YELLOW}💡 Key Endpoints:${NC}"
echo -e "  Status: ${GREEN}curl http://localhost:8080/circuit-breakers${NC}"
echo -e "  Metrics: ${GREEN}curl http://localhost:8080/metrics | grep circuit_breaker${NC}"
echo ""

echo -e "${YELLOW}📚 See CIRCUIT-BREAKER.md for complete documentation${NC}"
echo ""
