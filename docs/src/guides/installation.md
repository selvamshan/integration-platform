# Installation

## Docker (Recommended)

### Requirements

- Docker Engine 24+
- Docker Compose v2+
- 4 GB RAM minimum (8 GB recommended)
- 10 GB disk space

### Start All Services

```bash
git clone https://github.com/selvaraj-s4/integration-platform
cd integration-platform
./build.sh
```

The script starts:

| Service | Port | Purpose |
|---------|------|---------|
| Control Plane | 8081 | Flow/connector management |
| Data Plane | 8080 | Flow execution & HTTP triggers |
| PostgreSQL | 5432 | Persistent storage |
| NATS | 4222 | Event bus |
| Redis | 6379 | Rate limiting & caching |
| Keycloak | 8082 | Authentication & SSO |
| Prometheus | 9090 | Metrics collection |

## Building from Source

### Requirements

- Rust 1.75+ (`rustup update stable`)
- PostgreSQL 15+ client libraries
- Oracle Instant Client (optional, for Oracle connector)

### Build

```bash
cargo build --release
```

### Environment Variables

Create a `.env` file in the project root:

```env
# PostgreSQL
DATABASE_URL=postgres://user:password@localhost:5432/integration_platform

# NATS
NATS_URL=nats://localhost:4222

# Redis
REDIS_URL=redis://localhost:6379

# Keycloak
KEYCLOAK_URL=http://localhost:8082
KEYCLOAK_REALM=integration-platform
KEYCLOAK_CLIENT_ID=control-plane

# Secrets
JWT_SECRET=your-secret-key
ENCRYPTION_KEY=your-32-byte-hex-key
```

### Run Services

```bash
# Control Plane
RUST_LOG=info cargo run -p control-plane

# Data Plane (separate terminal)
RUST_LOG=info cargo run -p data-plane
```

## Verify Installation

```bash
curl http://localhost:8081/health
curl http://localhost:8080/health
```

Both should return `{"status":"healthy"}`.
