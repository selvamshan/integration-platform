# Flow Designer Enhancement Requirements

## Features to Implement

### 1. Node Delete Button
- X icon in top-right corner of each node
- Click to remove node from canvas

### 2. Right Panel - Node Properties Editor
When node is clicked, show properties in right panel:

#### HTTP Trigger Node
- Path input (text)
- Method dropdown (GET, POST, PUT, DELETE)

#### Transform Node
- Transform type dropdown (select, map, filter, etc.)
- Dynamic fields based on type
  - Select: fields array input
  - Filter: condition builder
  - Map: template editor
  - etc.

#### Connector Node
- Connector type dropdown (http, postgres, mysql)
- Connector instance dropdown (filtered by type)
- Operation dropdown (based on connector)
- Parameters editor (based on operation)

### 3. New API Endpoint Needed
```
GET /connector-instances/:connector_type
```

Returns connector instances filtered by type.

Example:
```bash
GET /connector-instances/postgres
→ Returns only PostgreSQL connector instances

GET /connector-instances/http
→ Returns only HTTP connector instances
```

---

## Implementation Plan

1. Create custom node component with delete button
2. Create NodePropertiesPanel component
3. Add node selection state
4. Add connector-instances filter endpoint
5. Integrate all pieces

Let's build it! 🚀
