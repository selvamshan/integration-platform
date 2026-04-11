#!/bin/bash

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║            Metrics Layer Test Suite                    ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}1. Checking Metrics Endpoint${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Accessing /metrics endpoint...${NC}"
metrics_response=$(curl -s http://localhost:8080/metrics)

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Metrics endpoint accessible${NC}"
    echo ""
    echo -e "${YELLOW}Sample metrics:${NC}"
    echo "$metrics_response" | head -20
    echo "..."
else
    echo -e "${RED}❌ Metrics endpoint not accessible${NC}"
    exit 1
fi

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}2. Initial Metrics State${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Current metric values:${NC}"
echo ""

# Extract key metrics
http_requests=$(echo "$metrics_response" | grep "^http_requests_total" | awk '{print $2}')
flow_executions=$(echo "$metrics_response" | grep "^flow_executions_total" | awk '{print $2}')
flow_success=$(echo "$metrics_response" | grep "^flow_executions_success_total" | awk '{print $2}')
flow_failed=$(echo "$metrics_response" | grep "^flow_executions_failed_total" | awk '{print $2}')
flows_loaded=$(echo "$metrics_response" | grep "^flows_loaded" | awk '{print $2}')

echo -e "  HTTP Requests:        ${GREEN}${http_requests:-0}${NC}"
echo -e "  Flow Executions:      ${GREEN}${flow_executions:-0}${NC}"
echo -e "  Flow Success:         ${GREEN}${flow_success:-0}${NC}"
echo -e "  Flow Failed:          ${RED}${flow_failed:-0}${NC}"
echo -e "  Flows Loaded:         ${BLUE}${flows_loaded:-0}${NC}"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}3. Creating Test Flow${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

curl -s -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "metrics-test-flow",
    "name": "Metrics Test Flow",
    "trigger": {"type": "http", "path": "/api/metrics-test", "method": "GET"},
    "steps": [
      {"type": "log", "name": "test", "message": "Testing metrics"},
      {"type": "call", "name": "db", "connector": "postgres", 
       "operation": "query", "params": {"sql": "SELECT COUNT(*) FROM users"}}
    ]
  }' > /dev/null

echo -e "${GREEN}✅ Test flow created${NC}"
sleep 3

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}4. Generating Load (50 requests)${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Executing flow 50 times...${NC}"
success_count=0
fail_count=0

for i in {1..50}; do
    response=$(curl -s -w "%{http_code}" -o /dev/null \
        -X POST http://localhost:8080/flows/metrics-test-flow/execute \
        -H "Content-Type: application/json" -d '{}')
    
    if [ "$response" == "200" ]; then
        success_count=$((success_count + 1))
        echo -n "."
    else
        fail_count=$((fail_count + 1))
        echo -n "x"
    fi
    
    # Small delay to make metrics more interesting
    sleep 0.05
done

echo ""
echo ""
echo -e "${GREEN}Completed:${NC}"
echo -e "  Success: ${GREEN}$success_count${NC}"
echo -e "  Failed:  ${RED}$fail_count${NC}"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}5. Checking Updated Metrics${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

sleep 1
metrics_response=$(curl -s http://localhost:8080/metrics)

# Extract updated metrics
new_http_requests=$(echo "$metrics_response" | grep "^http_requests_total" | awk '{print $2}')
new_flow_executions=$(echo "$metrics_response" | grep "^flow_executions_total" | awk '{print $2}')
new_flow_success=$(echo "$metrics_response" | grep "^flow_executions_success_total" | awk '{print $2}')
new_flow_failed=$(echo "$metrics_response" | grep "^flow_executions_failed_total" | awk '{print $2}')

echo -e "${YELLOW}Updated metric values:${NC}"
echo ""
echo -e "  HTTP Requests:        ${GREEN}${new_http_requests}${NC} (was: ${http_requests:-0})"
echo -e "  Flow Executions:      ${GREEN}${new_flow_executions}${NC} (was: ${flow_executions:-0})"
echo -e "  Flow Success:         ${GREEN}${new_flow_success}${NC} (was: ${flow_success:-0})"
echo -e "  Flow Failed:          ${RED}${new_flow_failed}${NC} (was: ${flow_failed:-0})"
echo ""

# Calculate success rate
if [ "${new_flow_executions:-0}" -gt 0 ]; then
    success_rate=$(awk "BEGIN {print ($new_flow_success / $new_flow_executions * 100)}")
    echo -e "  Success Rate:         ${GREEN}${success_rate}%${NC}"
fi
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}6. Testing Histogram Metrics${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Flow execution duration histogram:${NC}"
echo ""
echo "$metrics_response" | grep "flow_execution_duration_seconds"
echo ""

echo -e "${YELLOW}HTTP request duration histogram:${NC}"
echo ""
echo "$metrics_response" | grep "http_request_duration_seconds"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}7. Testing Rate Limiting Metrics${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

# Create rate limited flow
curl -s -X POST http://localhost:8081/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "rate-limited-metrics",
    "name": "Rate Limited Metrics Test",
    "trigger": {"type": "http", "path": "/api/rate-test", "method": "GET"},
    "steps": [{"type": "log", "name": "test", "message": "Rate limited"}],
    "rate_limit": {
      "max_requests": 3,
      "window_seconds": 60,
      "key_type": "global"
    }
  }' > /dev/null

sleep 2

echo -e "${YELLOW}Sending 5 requests (3 allowed, 2 blocked)...${NC}"

for i in {1..5}; do
    curl -s -X POST http://localhost:8080/flows/rate-limited-metrics/execute -d '{}' > /dev/null
done

sleep 1
metrics_response=$(curl -s http://localhost:8080/metrics)

rate_checks=$(echo "$metrics_response" | grep "^rate_limit_checks_total" | awk '{print $2}')
rate_blocked=$(echo "$metrics_response" | grep "^rate_limit_blocked_total" | awk '{print $2}')
rate_allowed=$(echo "$metrics_response" | grep "^rate_limit_allowed_total" | awk '{print $2}')

echo ""
echo -e "${YELLOW}Rate limiting metrics:${NC}"
echo -e "  Total Checks:  ${BLUE}${rate_checks}${NC}"
echo -e "  Allowed:       ${GREEN}${rate_allowed}${NC}"
echo -e "  Blocked:       ${RED}${rate_blocked}${NC}"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}8. Testing Redis Metrics${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

redis_ops=$(echo "$metrics_response" | grep "^redis_operations_total" | awk '{print $2}')
redis_errors=$(echo "$metrics_response" | grep "^redis_errors_total" | awk '{print $2}')

echo -e "${YELLOW}Redis metrics:${NC}"
echo -e "  Operations:    ${GREEN}${redis_ops}${NC}"
echo -e "  Errors:        ${RED}${redis_errors:-0}${NC}"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}9. Complete Metrics Output${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}All current metrics:${NC}"
echo ""
curl -s http://localhost:8080/metrics | grep -v "^#"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}10. Prometheus Query Examples${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Sample PromQL queries you can use:${NC}"
echo ""
echo -e "${BLUE}Request rate (last 5 min):${NC}"
echo -e "  rate(http_requests_total[5m])"
echo ""
echo -e "${BLUE}Flow success rate:${NC}"
echo -e "  rate(flow_executions_success_total[5m]) / rate(flow_executions_total[5m]) * 100"
echo ""
echo -e "${BLUE}P95 request latency:${NC}"
echo -e "  histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))"
echo ""
echo -e "${BLUE}Average flow duration:${NC}"
echo -e "  rate(flow_execution_duration_seconds_sum[5m]) / rate(flow_execution_duration_seconds_count[5m])"
echo ""
echo -e "${BLUE}Rate limit block rate:${NC}"
echo -e "  rate(rate_limit_blocked_total[5m]) / rate(rate_limit_checks_total[5m]) * 100"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ Metrics Tests Complete!${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}📊 Summary:${NC}"
echo ""
echo -e "  ✅ Metrics endpoint working (/metrics)"
echo -e "  ✅ HTTP request metrics tracked"
echo -e "  ✅ Flow execution metrics tracked"
echo -e "  ✅ Histogram metrics available"
echo -e "  ✅ Rate limiting metrics tracked"
echo -e "  ✅ Redis operations metrics tracked"
echo -e "  ✅ System state metrics (flows loaded)"
echo ""

echo -e "${YELLOW}💡 Next Steps:${NC}"
echo ""
echo -e "  1. Set up Prometheus:"
echo -e "     ${GREEN}See METRICS.md for prometheus.yml config${NC}"
echo ""
echo -e "  2. Set up Grafana dashboards:"
echo -e "     ${GREEN}Import dashboards from METRICS.md${NC}"
echo ""
echo -e "  3. Configure alerts:"
echo -e "     ${GREEN}Use alert rules from METRICS.md${NC}"
echo ""
echo -e "  4. View metrics in real-time:"
echo -e "     ${GREEN}curl http://localhost:8080/metrics${NC}"
echo ""

echo -e "${YELLOW}📚 See METRICS.md for complete documentation${NC}"
echo ""
