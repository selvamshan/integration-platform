# Configuration

## Environment Variables

### Control Plane (`crates/control-plane`)

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | required | PostgreSQL connection string |
| `NATS_URL` | `nats://localhost:4222` | NATS server URL |
| `KEYCLOAK_URL` | required | Keycloak base URL |
| `KEYCLOAK_REALM` | `integration-platform` | Keycloak realm name |
| `KEYCLOAK_CLIENT_ID` | `control-plane` | Keycloak client ID |
| `JWT_SECRET` | required | Secret for signing internal JWTs |
| `ENCRYPTION_KEY` | required | 32-byte hex key for encrypting connector credentials |
| `PORT` | `8081` | HTTP listen port |
| `RUST_LOG` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |

### Data Plane (`crates/data-plane`)

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | required | PostgreSQL connection string |
| `NATS_URL` | `nats://localhost:4222` | NATS server URL |
| `REDIS_URL` | `redis://localhost:6379` | Redis connection string |
| `CONTROL_PLANE_URL` | `http://localhost:8081` | Control plane base URL |
| `PORT` | `8080` | HTTP listen port |
| `RUST_LOG` | `info` | Log level |

## Database Migrations

Migrations run automatically on startup via `sqlx migrate run`.

Migration files live in `crates/control-plane/migrations/`. To run manually:

```bash
sqlx migrate run --database-url postgres://user:pass@localhost/integration_platform
```

## Docker Compose Overrides

Override any service setting in `docker-compose.override.yml` (not committed to git):

```yaml
services:
  data-plane:
    environment:
      RUST_LOG: debug
      REDIS_URL: redis://custom-redis:6379
```

## Keycloak Setup

See the full [RBAC & Keycloak guide](./rbac.md) for realm configuration, client setup, and role definitions.

## TLS / HTTPS

For production, terminate TLS at a reverse proxy (nginx, Caddy, or AWS ALB) in front of the platform services. The Rust services do not handle TLS directly.
