# HTTP Connector

The HTTP connector calls external REST APIs from within a flow. It supports five authentication types and full CRUD operations.

## Register a Connector Instance

```bash
curl -X POST http://localhost:8081/connector-instances \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "id": "my_api",
    "name": "My API",
    "connector_type": "http",
    "extra_attributes": {
      "base_url": "https://api.example.com",
      "timeout_ms": 30000,
      "default_headers": {
        "Content-Type": "application/json"
      }
    }
  }'
```

## Authentication Types

### None (public APIs)

```json
{ "extra_attributes": { "base_url": "https://api.example.com" } }
```

### Bearer Token

```json
{
  "extra_attributes": {
    "base_url": "https://api.github.com",
    "auth": { "type": "bearer", "token": "ghp_xxxx" }
  }
}
```

### Basic Auth

```json
{
  "extra_attributes": {
    "auth": { "type": "basic", "username": "user", "password": "pass" }
  }
}
```

### API Key

```json
{
  "extra_attributes": {
    "auth": { "type": "apikey", "header_name": "X-API-Key", "api_key": "key" }
  }
}
```

### OAuth2 (Client Credentials)

```json
{
  "extra_attributes": {
    "base_url": "https://api.salesforce.com",
    "auth": {
      "type": "oauth2",
      "token_url": "https://login.salesforce.com/services/oauth2/token",
      "client_id": "client_id",
      "client_secret": "client_secret",
      "scope": "api"
    }
  }
}
```

OAuth tokens are cached in memory and reused for subsequent requests.

## Flow Step Operations

### GET

```json
{
  "type": "call",
  "name": "fetch_user",
  "connector": "my_api",
  "operation": "get",
  "params": { "path": "/users/{{trigger.body.id}}" }
}
```

### POST

```json
{
  "type": "call",
  "name": "create_issue",
  "connector": "github_api",
  "operation": "post",
  "params": {
    "path": "/repos/owner/repo/issues",
    "body": { "title": "{{trigger.body.title}}" }
  }
}
```

### PUT / PATCH / DELETE

```json
{ "operation": "put",    "params": { "path": "/users/1", "body": { ... } } }
{ "operation": "patch",  "params": { "path": "/users/1", "body": { ... } } }
{ "operation": "delete", "params": { "path": "/users/1" } }
```

## Configuration Reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `base_url` | string | null | Prepended to all request paths |
| `auth` | object | null | Authentication configuration |
| `default_headers` | object | `{}` | Headers added to every request |
| `timeout_ms` | number | 30000 | Request timeout in milliseconds |

## Step Response

```json
{
  "status": 200,
  "data": { ... }
}
```

Reference in subsequent steps: `{{step_name.data.field}}` or `{{step_name.status}}`.
