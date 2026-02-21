# JSON Transformation Engine — Complete Guide

Powerful data transformation capabilities for integration flows with 11 transformation types.

---

## Features

✅ **11 Transformation Types:**
1. Select — Pick specific fields
2. Map — Transform array elements
3. Filter — Filter array based on conditions
4. Flatten — Flatten nested objects
5. Group — Group array by field
6. Rename — Rename fields
7. Merge — Merge multiple objects
8. Split — Split strings into arrays
9. Convert — Type conversions
10. Conditional — If/then/else logic
11. Template — Variable substitution

✅ **Variable Substitution:** `{{fieldName}}` syntax

✅ **Conditional Logic:** eq, ne, gt, gte, lt, lte, contains, in

✅ **Type Conversions:** string, number, boolean, array

---

## Usage in Flows

### Transform Step

```json
{
  "type": "transform",
  "name": "transform_data",
  "spec": {
    "type": "select",
    "fields": ["name", "email", "age"]
  }
}
```

**Input:** Previous step's output or trigger data  
**Output:** Transformed data available as `{{transform_data}}`

---

## Transformation Types

### 1. Select — Pick Specific Fields

**Remove unwanted fields, keep only what you need.**

```json
{
  "type": "select",
  "fields": ["name", "email", "age"]
}
```

**Example:**
```json
// Input
{
  "name": "John Doe",
  "email": "john@example.com",
  "age": 30,
  "password": "secret123",
  "ssn": "123-45-6789"
}

// Output
{
  "name": "John Doe",
  "email": "john@example.com",
  "age": 30
}
```

**Use cases:**
- Remove sensitive fields before logging
- API response filtering
- Data privacy compliance

---

### 2. Map — Transform Array Elements

**Apply template to each array element.**

```json
{
  "type": "map",
  "template": {
    "id": "{{id}}",
    "fullName": "{{firstName}} {{lastName}}",
    "isAdult": "{{age}}"
  }
}
```

**Example:**
```json
// Input
[
  {"id": 1, "firstName": "John", "lastName": "Doe", "age": 30},
  {"id": 2, "firstName": "Jane", "lastName": "Smith", "age": 25}
]

// Output
[
  {"id": 1, "fullName": "John Doe", "isAdult": "30"},
  {"id": 2, "fullName": "Jane Smith", "isAdult": "25"}
]
```

**Use cases:**
- Reshape API responses
- Combine fields
- Format data for display

---

### 3. Filter — Filter Array by Condition

**Keep only elements matching condition.**

```json
{
  "type": "filter",
  "condition": {
    "field": "age",
    "op": "gte",
    "value": 18
  }
}
```

**Operators:**
- `eq` — Equal
- `ne` — Not equal
- `gt` — Greater than
- `gte` — Greater than or equal
- `lt` — Less than
- `lte` — Less than or equal
- `contains` — String contains
- `in` — Value in array

**Example:**
```json
// Input
[
  {"name": "John", "age": 25, "active": true},
  {"name": "Jane", "age": 17, "active": true},
  {"name": "Bob", "age": 30, "active": false}
]

// Filter: age >= 18
// Output
[
  {"name": "John", "age": 25, "active": true},
  {"name": "Bob", "age": 30, "active": false}
]
```

**Use cases:**
- Remove invalid data
- Filter by status
- Age verification

---

### 4. Flatten — Flatten Nested Objects

**Convert nested objects to flat structure.**

```json
{
  "type": "flatten",
  "separator": "_"
}
```

**Example:**
```json
// Input
{
  "user": {
    "name": "John",
    "address": {
      "city": "NYC",
      "zip": "10001"
    }
  },
  "order": {
    "id": 123,
    "total": 99.99
  }
}

// Output (separator: "_")
{
  "user_name": "John",
  "user_address_city": "NYC",
  "user_address_zip": "10001",
  "order_id": 123,
  "order_total": 99.99
}
```

**Use cases:**
- Database inserts (flat schema)
- CSV exports
- Analytics data

---

### 5. Group — Group Array by Field

**Group array elements by a field value.**

```json
{
  "type": "group",
  "by": "category"
}
```

**Example:**
```json
// Input
[
  {"name": "Apple", "category": "fruit"},
  {"name": "Carrot", "category": "vegetable"},
  {"name": "Banana", "category": "fruit"}
]

// Output
{
  "\"fruit\"": [
    {"name": "Apple", "category": "fruit"},
    {"name": "Banana", "category": "fruit"}
  ],
  "\"vegetable\"": [
    {"name": "Carrot", "category": "vegetable"}
  ]
}
```

**Use cases:**
- Analytics aggregation
- Report grouping
- Data categorization

---

### 6. Rename — Rename Fields

**Rename object fields.**

```json
{
  "type": "rename",
  "mapping": {
    "firstName": "first_name",
    "lastName": "last_name",
    "age": "user_age"
  }
}
```

**Example:**
```json
// Input
{
  "firstName": "John",
  "lastName": "Doe",
  "age": 30,
  "email": "john@example.com"
}

// Output
{
  "first_name": "John",
  "last_name": "Doe",
  "user_age": 30,
  "email": "john@example.com"
}
```

**Use cases:**
- API format conversion (camelCase ↔ snake_case)
- Database schema mapping
- Third-party integrations

---

### 7. Merge — Merge Multiple Objects

**Combine multiple objects into one.**

```json
{
  "type": "merge",
  "sources": [
    "{{additional_data}}",
    "{{metadata}}"
  ]
}
```

**Example:**
```json
// Input (base)
{"id": 1, "name": "John"}

// Additional sources
Source 1: {"email": "john@example.com", "age": 30}
Source 2: {"country": "USA", "verified": true}

// Output
{
  "id": 1,
  "name": "John",
  "email": "john@example.com",
  "age": 30,
  "country": "USA",
  "verified": true
}
```

**Use cases:**
- Combine data from multiple APIs
- Enrich data with metadata
- Build complete records

---

### 8. Split — Split String to Array

**Split a string field into array.**

```json
{
  "type": "split",
  "field": "tags",
  "delimiter": ","
}
```

**Example:**
```json
// Input
{
  "title": "My Post",
  "tags": "javascript, nodejs, api"
}

// Output
{
  "title": "My Post",
  "tags": ["javascript", "nodejs", "api"]
}
```

**Use cases:**
- Parse CSV fields
- Tag processing
- Multi-value fields

---

### 9. Convert — Type Conversions

**Convert field types.**

```json
{
  "type": "convert",
  "fields": {
    "age": "number",
    "active": "boolean",
    "tags": "array"
  }
}
```

**Supported types:**
- `string` — Convert to string
- `number` — Convert to number
- `boolean` — Convert to boolean (true/false, 1/0, yes/no)
- `array` — Wrap in array (if not already)

**Example:**
```json
// Input
{
  "name": "John",
  "age": "30",
  "active": "true",
  "score": "95.5"
}

// Output
{
  "name": "John",
  "age": 30,
  "active": true,
  "score": 95.5
}
```

**Use cases:**
- Fix type mismatches
- Database type compliance
- JSON schema validation

---

### 10. Conditional — If/Then/Else Logic

**Apply transformation based on condition.**

```json
{
  "type": "conditional",
  "if": {
    "field": "age",
    "op": "gte",
    "value": 18
  },
  "then": {
    "status": "adult",
    "canVote": true
  },
  "else": {
    "status": "minor",
    "canVote": false
  }
}
```

**Example:**
```json
// Input
{"name": "John", "age": 25}

// Output (age >= 18)
{
  "status": "adult",
  "canVote": true
}

// Input
{"name": "Jane", "age": 16}

// Output (age < 18)
{
  "status": "minor",
  "canVote": false
}
```

**Use cases:**
- Status determination
- Eligibility checks
- Dynamic field values

---

### 11. Template — Variable Substitution

**Apply template with variable substitution.**

```json
{
  "type": "template",
  "template": {
    "fullName": "{{firstName}} {{lastName}}",
    "greeting": "Hello, {{firstName}}!",
    "age": "{{age}}"
  }
}
```

**Example:**
```json
// Input
{
  "firstName": "John",
  "lastName": "Doe",
  "age": 30
}

// Output
{
  "fullName": "John Doe",
  "greeting": "Hello, John!",
  "age": "30"
}
```

**Use cases:**
- Format messages
- Build URLs
- Create computed fields

---

## Flow Examples

### Example 1: Clean API Response

**Scenario:** Remove sensitive fields from user data

```json
{
  "id": "clean-user-data",
  "trigger": {
    "type": "http",
    "path": "/users/:id",
    "method": "GET"
  },
  "steps": [
    {
      "type": "call",
      "name": "get_user",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users WHERE id = $1",
        "params": ["{{trigger.params.id}}"]
      }
    },
    {
      "type": "transform",
      "name": "clean_data",
      "spec": {
        "type": "select",
        "fields": ["id", "name", "email", "created_at"]
      }
    }
  ]
}
```

---

### Example 2: Format User List

**Scenario:** Transform array of users to display format

```json
{
  "id": "format-users",
  "trigger": {
    "type": "http",
    "path": "/users",
    "method": "GET"
  },
  "steps": [
    {
      "type": "call",
      "name": "get_users",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT first_name, last_name, age, email FROM users"
      }
    },
    {
      "type": "transform",
      "name": "format_users",
      "spec": {
        "type": "map",
        "template": {
          "name": "{{first_name}} {{last_name}}",
          "contact": "{{email}}",
          "age": "{{age}}"
        }
      }
    }
  ]
}
```

---

### Example 3: Filter and Transform

**Scenario:** Get active adult users and format them

```json
{
  "steps": [
    {
      "type": "call",
      "name": "get_users",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users"
      }
    },
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
    },
    {
      "type": "transform",
      "name": "format_data",
      "spec": {
        "type": "map",
        "template": {
          "userId": "{{id}}",
          "fullName": "{{firstName}} {{lastName}}",
          "contact": "{{email}}"
        }
      }
    }
  ]
}
```

---

### Example 4: Flatten for Database Insert

**Scenario:** Flatten nested API response before database insert

```json
{
  "steps": [
    {
      "type": "call",
      "name": "fetch_order",
      "connector": "shopify_api",
      "operation": "get",
      "params": {
        "path": "/orders/123"
      }
    },
    {
      "type": "transform",
      "name": "flatten_order",
      "spec": {
        "type": "flatten",
        "separator": "_"
      }
    },
    {
      "type": "call",
      "name": "insert_order",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "INSERT INTO orders (customer_name, customer_email, shipping_city) VALUES ($1, $2, $3)",
        "params": [
          "{{flatten_order.customer_name}}",
          "{{flatten_order.customer_email}}",
          "{{flatten_order.shipping_address_city}}"
        ]
      }
    }
  ]
}
```

---

### Example 5: Conditional Status

**Scenario:** Set user status based on age

```json
{
  "steps": [
    {
      "type": "call",
      "name": "get_user",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users WHERE id = $1",
        "params": ["{{trigger.body.user_id}}"]
      }
    },
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
          "status": "adult",
          "eligible_for_voting": true
        },
        "else": {
          "status": "minor",
          "eligible_for_voting": false
        }
      }
    }
  ]
}
```

---

### Example 6: Type Conversion

**Scenario:** Fix type mismatches from CSV import

```json
{
  "steps": [
    {
      "type": "transform",
      "name": "convert_types",
      "spec": {
        "type": "convert",
        "fields": {
          "age": "number",
          "salary": "number",
          "active": "boolean",
          "tags": "array"
        }
      }
    },
    {
      "type": "call",
      "name": "insert_user",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "INSERT INTO users (name, age, salary, active) VALUES ($1, $2, $3, $4)",
        "params": [
          "{{convert_types.name}}",
          "{{convert_types.age}}",
          "{{convert_types.salary}}",
          "{{convert_types.active}}"
        ]
      }
    }
  ]
}
```

---

### Example 7: Rename for API Compatibility

**Scenario:** Convert between camelCase and snake_case

```json
{
  "steps": [
    {
      "type": "call",
      "name": "get_data",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT first_name, last_name, email_address FROM users WHERE id = $1",
        "params": ["1"]
      }
    },
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
    },
    {
      "type": "call",
      "name": "send_to_api",
      "connector": "external_api",
      "operation": "post",
      "params": {
        "path": "/users",
        "body": "{{rename_fields}}"
      }
    }
  ]
}
```

---

### Example 8: Split Tags

**Scenario:** Convert comma-separated tags to array

```json
{
  "steps": [
    {
      "type": "call",
      "name": "get_post",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT title, tags FROM posts WHERE id = $1",
        "params": ["123"]
      }
    },
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
}
```

---

### Example 9: Group Orders by Status

**Scenario:** Group orders for analytics

```json
{
  "steps": [
    {
      "type": "call",
      "name": "get_orders",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM orders WHERE created_at >= NOW() - INTERVAL '7 days'"
      }
    },
    {
      "type": "transform",
      "name": "group_by_status",
      "spec": {
        "type": "group",
        "by": "status"
      }
    }
  ]
}
```

---

### Example 10: Merge User Data

**Scenario:** Combine data from multiple sources

```json
{
  "steps": [
    {
      "type": "call",
      "name": "get_user",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM users WHERE id = $1",
        "params": ["1"]
      }
    },
    {
      "type": "call",
      "name": "get_preferences",
      "connector": "postgres_prod",
      "operation": "query",
      "params": {
        "sql": "SELECT * FROM user_preferences WHERE user_id = $1",
        "params": ["1"]
      }
    },
    {
      "type": "transform",
      "name": "merge_data",
      "spec": {
        "type": "merge",
        "sources": [
          "{{get_preferences}}"
        ]
      }
    }
  ]
}
```

---

## Chaining Transformations

**Multiple transformations in sequence:**

```json
{
  "steps": [
    {
      "type": "call",
      "name": "get_users",
      "connector": "api",
      "operation": "get",
      "params": {"path": "/users"}
    },
    {
      "type": "transform",
      "name": "step1_filter",
      "spec": {
        "type": "filter",
        "condition": {"field": "active", "op": "eq", "value": true}
      }
    },
    {
      "type": "transform",
      "name": "step2_map",
      "spec": {
        "type": "map",
        "template": {
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
}
```

---

## Best Practices

### 1. Chain Simple Transformations
✅ Do: Multiple simple transforms
```json
[
  {"type": "filter", ...},
  {"type": "map", ...},
  {"type": "select", ...}
]
```

❌ Don't: Complex nested logic

### 2. Use Descriptive Names
✅ Do: `"name": "filter_active_users"`
❌ Don't: `"name": "transform1"`

### 3. Type Safety
✅ Do: Convert types explicitly
```json
{"type": "convert", "fields": {"age": "number"}}
```

❌ Don't: Rely on implicit conversions

### 4. Test Incrementally
✅ Do: Test each transform step
❌ Don't: Chain many untested transforms

---

## Performance Tips

1. **Filter early** — Reduce data before mapping
2. **Select late** — Remove fields after all processing
3. **Avoid deep nesting** — Flatten when possible
4. **Batch operations** — Use arrays efficiently

---

## Error Handling

**Transform errors return:**
```json
{
  "error": "Transform error: Invalid transform type: xyz"
}
```

**Common errors:**
- Invalid transform type
- Missing required fields
- Type conversion failures
- Invalid operators
- Array expected but object provided

---

## Summary

✅ **11 transformation types** covering all common data manipulation needs  
✅ **Variable substitution** with `{{field}}` syntax  
✅ **Conditional logic** with 8 comparison operators  
✅ **Type conversions** for string, number, boolean, array  
✅ **Chainable** for complex transformations  
✅ **Production-ready** with comprehensive error handling  

**Transform any data shape into the format you need!** 🔄✨✅
