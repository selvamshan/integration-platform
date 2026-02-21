#!/bin/bash
# Test Transformation Engine in Flows

set -e

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'

CP="http://localhost:8081"
DP="http://localhost:8080"

banner() { echo -e "\n${BLUE}${BOLD}══════════════════════════════════════════${NC}"; echo -e "${BLUE}${BOLD}  $1${NC}"; echo -e "${BLUE}${BOLD}══════════════════════════════════════════${NC}"; }
ok()     { echo -e "${GREEN}✅ $1${NC}"; }
fail()   { echo -e "${RED}❌ $1${NC}"; exit 1; }
info()   { echo -e "${YELLOW}   $1${NC}"; }

banner "Transformation Engine Test Suite"

echo ""
info "Testing 11 transformation types in flows"
echo ""

# ─── Test 1: Select Transform ─────────────────────────────────────────────────
banner "1. Select Transform (Remove Sensitive Fields)"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-select",
    "name": "Test Select Transform",
    "trigger": {
      "type": "http",
      "path": "/test-select",
      "method": "POST"
    },
    "steps": [
      {
        "type": "transform",
        "name": "remove_sensitive",
        "spec": {
          "type": "select",
          "fields": ["name", "email", "age"]
        }
      }
    ]
  }' | jq '.' || true

ok "Created test-select flow"

info "Testing..."
RESULT=$(curl -s -X POST $DP/api/trigger/test-select \
  -H "Content-Type: application/json" \
  -d '{
    "name": "John Doe",
    "email": "john@example.com",
    "age": 30,
    "password": "secret123",
    "ssn": "123-45-6789"
  }')

echo "$RESULT" | jq '.'
if echo "$RESULT" | jq -e '.name' > /dev/null && ! echo "$RESULT" | jq -e '.password' > /dev/null; then
  ok "Select transform works - sensitive fields removed"
else
  fail "Select transform failed"
fi

# ─── Test 2: Rename Transform ─────────────────────────────────────────────────
banner "2. Rename Transform (snake_case to camelCase)"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-rename",
    "name": "Test Rename Transform",
    "trigger": {
      "type": "http",
      "path": "/test-rename",
      "method": "POST"
    },
    "steps": [
      {
        "type": "transform",
        "name": "rename_fields",
        "spec": {
          "type": "rename",
          "mapping": {
            "first_name": "firstName",
            "last_name": "lastName",
            "email_address": "email"
          }
        }
      }
    ]
  }' | jq '.' || true

ok "Created test-rename flow"

info "Testing..."
RESULT=$(curl -s -X POST $DP/api/trigger/test-rename \
  -H "Content-Type: application/json" \
  -d '{
    "first_name": "Jane",
    "last_name": "Smith",
    "email_address": "jane@example.com"
  }')

echo "$RESULT" | jq '.'
if echo "$RESULT" | jq -e '.firstName' > /dev/null && echo "$RESULT" | jq -e '.lastName' > /dev/null; then
  ok "Rename transform works"
else
  fail "Rename transform failed"
fi

# ─── Test 3: Filter Transform ─────────────────────────────────────────────────
banner "3. Filter Transform (Age >= 18)"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-filter",
    "name": "Test Filter Transform",
    "trigger": {
      "type": "http",
      "path": "/test-filter",
      "method": "POST"
    },
    "steps": [
      {
        "type": "transform",
        "name": "filter_adults",
        "spec": {
          "type": "filter",
          "condition": {
            "field": "age",
            "op": "gte",
            "value": 18
          }
        }
      }
    ]
  }' | jq '.' || true

ok "Created test-filter flow"

info "Testing..."
RESULT=$(curl -s -X POST $DP/api/trigger/test-filter \
  -H "Content-Type: application/json" \
  -d '[
    {"name": "John", "age": 25},
    {"name": "Jane", "age": 17},
    {"name": "Bob", "age": 30}
  ]')

echo "$RESULT" | jq '.'
COUNT=$(echo "$RESULT" | jq '. | length')
if [ "$COUNT" == "2" ]; then
  ok "Filter transform works - filtered 2 adults"
else
  fail "Filter transform failed - expected 2, got $COUNT"
fi

# ─── Test 4: Map Transform ────────────────────────────────────────────────────
banner "4. Map Transform (Combine Fields)"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-map",
    "name": "Test Map Transform",
    "trigger": {
      "type": "http",
      "path": "/test-map",
      "method": "POST"
    },
    "steps": [
      {
        "type": "transform",
        "name": "format_users",
        "spec": {
          "type": "map",
          "template": {
            "id": "{{id}}",
            "fullName": "{{firstName}} {{lastName}}",
            "contact": "{{email}}"
          }
        }
      }
    ]
  }' | jq '.' || true

ok "Created test-map flow"

info "Testing..."
RESULT=$(curl -s -X POST $DP/api/trigger/test-map \
  -H "Content-Type: application/json" \
  -d '[
    {"id": 1, "firstName": "John", "lastName": "Doe", "email": "john@example.com"},
    {"id": 2, "firstName": "Jane", "lastName": "Smith", "email": "jane@example.com"}
  ]')

echo "$RESULT" | jq '.'
if echo "$RESULT" | jq -e '.[0].fullName' > /dev/null; then
  ok "Map transform works"
else
  fail "Map transform failed"
fi

# ─── Test 5: Flatten Transform ────────────────────────────────────────────────
banner "5. Flatten Transform"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-flatten",
    "name": "Test Flatten Transform",
    "trigger": {
      "type": "http",
      "path": "/test-flatten",
      "method": "POST"
    },
    "steps": [
      {
        "type": "transform",
        "name": "flatten_data",
        "spec": {
          "type": "flatten",
          "separator": "_"
        }
      }
    ]
  }' | jq '.' || true

ok "Created test-flatten flow"

info "Testing..."
RESULT=$(curl -s -X POST $DP/api/trigger/test-flatten \
  -H "Content-Type: application/json" \
  -d '{
    "user": {
      "name": "John",
      "address": {
        "city": "NYC",
        "zip": "10001"
      }
    }
  }')

echo "$RESULT" | jq '.'
if echo "$RESULT" | jq -e '.user_name' > /dev/null && echo "$RESULT" | jq -e '.user_address_city' > /dev/null; then
  ok "Flatten transform works"
else
  fail "Flatten transform failed"
fi

# ─── Test 6: Convert Transform ────────────────────────────────────────────────
banner "6. Convert Transform (Type Conversion)"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-convert",
    "name": "Test Convert Transform",
    "trigger": {
      "type": "http",
      "path": "/test-convert",
      "method": "POST"
    },
    "steps": [
      {
        "type": "transform",
        "name": "convert_types",
        "spec": {
          "type": "convert",
          "fields": {
            "age": "number",
            "active": "boolean"
          }
        }
      }
    ]
  }' | jq '.' || true

ok "Created test-convert flow"

info "Testing..."
RESULT=$(curl -s -X POST $DP/api/trigger/test-convert \
  -H "Content-Type: application/json" \
  -d '{
    "name": "John",
    "age": "30",
    "active": "true"
  }')

echo "$RESULT" | jq '.'
AGE_TYPE=$(echo "$RESULT" | jq -r '.age | type')
ACTIVE_TYPE=$(echo "$RESULT" | jq -r '.active | type')
if [ "$AGE_TYPE" == "number" ] && [ "$ACTIVE_TYPE" == "boolean" ]; then
  ok "Convert transform works - types converted"
else
  fail "Convert transform failed - age: $AGE_TYPE, active: $ACTIVE_TYPE"
fi

# ─── Test 7: Conditional Transform ────────────────────────────────────────────
banner "7. Conditional Transform (If/Then/Else)"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-conditional",
    "name": "Test Conditional Transform",
    "trigger": {
      "type": "http",
      "path": "/test-conditional",
      "method": "POST"
    },
    "steps": [
      {
        "type": "transform",
        "name": "add_status",
        "spec": {
          "type": "conditional",
          "if": {
            "field": "age",
            "op": "gte",
            "value": 18
          },
          "then": {
            "status": "adult"
          },
          "else": {
            "status": "minor"
          }
        }
      }
    ]
  }' | jq '.' || true

ok "Created test-conditional flow"

info "Testing adult..."
RESULT=$(curl -s -X POST $DP/api/trigger/test-conditional \
  -H "Content-Type: application/json" \
  -d '{"name": "John", "age": 25}')

echo "$RESULT" | jq '.'
STATUS=$(echo "$RESULT" | jq -r '.status')
if [ "$STATUS" == "adult" ]; then
  ok "Conditional transform works (adult)"
else
  fail "Conditional transform failed - got: $STATUS"
fi

# ─── Test 8: Template Transform ───────────────────────────────────────────────
banner "8. Template Transform (Variable Substitution)"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-template",
    "name": "Test Template Transform",
    "trigger": {
      "type": "http",
      "path": "/test-template",
      "method": "POST"
    },
    "steps": [
      {
        "type": "transform",
        "name": "apply_template",
        "spec": {
          "type": "template",
          "template": {
            "fullName": "{{firstName}} {{lastName}}",
            "greeting": "Hello, {{firstName}}!",
            "summary": "{{firstName}} is {{age}} years old"
          }
        }
      }
    ]
  }' | jq '.' || true

ok "Created test-template flow"

info "Testing..."
RESULT=$(curl -s -X POST $DP/api/trigger/test-template \
  -H "Content-Type: application/json" \
  -d '{
    "firstName": "John",
    "lastName": "Doe",
    "age": 30
  }')

echo "$RESULT" | jq '.'
if echo "$RESULT" | jq -e '.fullName' > /dev/null && echo "$RESULT" | jq -e '.greeting' > /dev/null; then
  ok "Template transform works"
else
  fail "Template transform failed"
fi

# ─── Test 9: Split Transform ──────────────────────────────────────────────────
banner "9. Split Transform (String to Array)"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-split",
    "name": "Test Split Transform",
    "trigger": {
      "type": "http",
      "path": "/test-split",
      "method": "POST"
    },
    "steps": [
      {
        "type": "transform",
        "name": "split_tags",
        "spec": {
          "type": "split",
          "field": "tags",
          "delimiter": ","
        }
      }
    ]
  }' | jq '.' || true

ok "Created test-split flow"

info "Testing..."
RESULT=$(curl -s -X POST $DP/api/trigger/test-split \
  -H "Content-Type: application/json" \
  -d '{
    "title": "My Post",
    "tags": "javascript,nodejs,api"
  }')

echo "$RESULT" | jq '.'
TAGS_TYPE=$(echo "$RESULT" | jq -r '.tags | type')
if [ "$TAGS_TYPE" == "array" ]; then
  ok "Split transform works - string converted to array"
else
  fail "Split transform failed - type: $TAGS_TYPE"
fi

# ─── Test 10: Chained Transforms ──────────────────────────────────────────────
banner "10. Chained Transforms (Filter → Map → Select)"

curl -s -X POST $CP/flows \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-chained",
    "name": "Test Chained Transforms",
    "trigger": {
      "type": "http",
      "path": "/test-chained",
      "method": "POST"
    },
    "steps": [
      {
        "type": "transform",
        "name": "step1_filter",
        "spec": {
          "type": "filter",
          "condition": {
            "field": "active",
            "op": "eq",
            "value": true
          }
        }
      },
      {
        "type": "transform",
        "name": "step2_map",
        "spec": {
          "type": "map",
          "template": {
            "id": "{{id}}",
            "name": "{{firstName}} {{lastName}}",
            "email": "{{email}}"
          }
        }
      },
      {
        "type": "transform",
        "name": "step3_select",
        "spec": {
          "type": "select",
          "fields": ["name", "email"]
        }
      }
    ]
  }' | jq '.' || true

ok "Created test-chained flow"

info "Testing..."
RESULT=$(curl -s -X POST $DP/api/trigger/test-chained \
  -H "Content-Type: application/json" \
  -d '[
    {"id": 1, "firstName": "John", "lastName": "Doe", "email": "john@example.com", "active": true},
    {"id": 2, "firstName": "Jane", "lastName": "Smith", "email": "jane@example.com", "active": false},
    {"id": 3, "firstName": "Bob", "lastName": "Johnson", "email": "bob@example.com", "active": true}
  ]')

echo "$RESULT" | jq '.'
# Should have 2 items (filtered), with name and email fields (selected)
COUNT=$(echo "$RESULT" | jq '. | length')
HAS_NAME=$(echo "$RESULT" | jq -e '.[0].name' > /dev/null && echo "yes" || echo "no")
HAS_ID=$(echo "$RESULT" | jq -e '.[0].id' > /dev/null && echo "yes" || echo "no")

if [ "$COUNT" == "2" ] && [ "$HAS_NAME" == "yes" ] && [ "$HAS_ID" == "no" ]; then
  ok "Chained transforms work - filter → map → select"
else
  fail "Chained transforms failed"
fi

# ─── Summary ──────────────────────────────────────────────────────────────────
banner "✅ Test Summary"

echo ""
echo -e "${BOLD}Transformation Types Tested:${NC}"
echo -e "  ${GREEN}✓${NC} Select (remove fields)"
echo -e "  ${GREEN}✓${NC} Rename (field mapping)"
echo -e "  ${GREEN}✓${NC} Filter (array filtering)"
echo -e "  ${GREEN}✓${NC} Map (array transformation)"
echo -e "  ${GREEN}✓${NC} Flatten (nested objects)"
echo -e "  ${GREEN}✓${NC} Convert (type conversion)"
echo -e "  ${GREEN}✓${NC} Conditional (if/then/else)"
echo -e "  ${GREEN}✓${NC} Template (variable substitution)"
echo -e "  ${GREEN}✓${NC} Split (string to array)"
echo -e "  ${GREEN}✓${NC} Chained (multiple transforms)"
echo ""
echo -e "${BOLD}Flows Created:${NC}"
echo -e "  • test-select"
echo -e "  • test-rename"
echo -e "  • test-filter"
echo -e "  • test-map"
echo -e "  • test-flatten"
echo -e "  • test-convert"
echo -e "  • test-conditional"
echo -e "  • test-template"
echo -e "  • test-split"
echo -e "  • test-chained"
echo ""
echo -e "${GREEN}${BOLD}All transformation tests passed! 🔄✨✅${NC}"
echo ""
