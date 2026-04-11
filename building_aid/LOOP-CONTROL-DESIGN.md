# Loop Control for Flows - Complete Design

## Problem Statement

How to handle looped flow steps for pagination - fetching data using HTTP connector with paginated cursor, inserting each page into database, looping until no more pages exist.

---

## Solution: Loop Control Step Type

### Loop Types Supported

1. **While Loop** - Condition-based iteration
2. **For-Each Loop** - Array iteration  
3. **Count Loop** - Fixed iterations

---

## Complete Example: Paginated API Sync

```json
{
  "id": "paginated-user-sync",
  "name": "Sync Paginated Users to Database",
  "trigger": {
    "type": "http",
    "path": "/sync-users",
    "method": "POST"
  },
  "steps": [
    {
      "type": "set_variable",
      "name": "init_cursor",
      "variables": {
        "cursor": null,
        "total_users": 0
      }
    },
    {
      "type": "loop",
      "name": "pagination_loop",
      "loop_mode": "while",
      "condition": {
        "type": "expression",
        "expression": "cursor != 'EOF'"
      },
      "max_iterations": 100,
      "steps": [
        {
          "type": "call",
          "name": "fetch_page",
          "connector": "http-api",
          "operation": "get",
          "params": {
            "url": "https://api.example.com/users",
            "query_params": {
              "cursor": "{{cursor}}",
              "limit": 100
            }
          }
        },
        {
          "type": "loop",
          "name": "insert_users",
          "loop_mode": "foreach",
          "iterate_over": "{{fetch_page.data}}",
          "steps": [
            {
              "type": "call",
              "name": "insert_user",
              "connector": "postgres",
              "operation": "execute",
              "params": {
                "sql": "INSERT INTO users (id, name, email) VALUES ($1, $2, $3) ON CONFLICT (id) DO UPDATE SET name = $2, email = $3",
                "params": ["{{item.id}}", "{{item.name}}", "{{item.email}}"]
              }
            }
          ]
        },
        {
          "type": "set_variable",
          "name": "update_cursor",
          "variables": {
            "cursor": "{{fetch_page.next_cursor || 'EOF'}}",
            "total_users": "{{total_users + fetch_page.data.length}}"
          }
        }
      ]
    },
    {
      "type": "log",
      "name": "completion",
      "message": "Synced {{total_users}} users in {{loop_iterations}} pages"
    }
  ]
}
```

---

## Loop Variables

Inside loop steps, these variables are available:

- `{{item}}` - Current item (for-each loops)
- `{{index}}` - Current iteration index (0-based)
- `{{iteration}}` - Current iteration number (1-based)
- `{{loop_iterations}}` - Total iterations after loop completes

---

## Loop Types in Detail

### 1. While Loop

Continue while condition is true:

```json
{
  "type": "loop",
  "loop_mode": "while",
  "condition": {
    "type": "expression",
    "expression": "cursor != 'EOF'"
  },
  "max_iterations": 100,
  "steps": [...]
}
```

### 2. For-Each Loop

Iterate over array:

```json
{
  "type": "loop",
  "loop_mode": "foreach",
  "iterate_over": "{{api_response.items}}",
  "steps": [...]
}
```

### 3. Count Loop

Fixed number of iterations:

```json
{
  "type": "loop",
  "loop_mode": "count",
  "count": 5,
  "steps": [...]
}
```

---

## Safety Features

- `max_iterations` - Prevent infinite loops (default: 1000)
- `break_when` - Early loop termination
- `continue_when` - Skip iterations
- `timeout` - Time limit

---

## Use Cases

1. **Paginated API Sync** - Fetch all pages
2. **Batch Processing** - Process array of items
3. **Retry Logic** - Retry with backoff
4. **Data Migration** - Move data between systems

---

## Implementation Status

✅ Design complete  
✅ Rust implementation created  
✅ Example flows provided  
⏳ Integration with runtime (next step)
