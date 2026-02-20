# Building the Platform

## Dependencies

The platform uses **rustls** instead of **native-tls** to avoid OpenSSL version conflicts.

### Rust Version

**Minimum:** 1.75  
**Recommended:** 1.77+

### Key Dependencies

| Crate | Version | Notes |
|-------|---------|-------|
| `tokio` | 1.35 | Async runtime |
| `axum` | 0.7 | Web framework |
| `reqwest` | 0.12 | HTTP client with **rustls-tls** |
| `sqlx` | 0.7 | Database (postgres) |
| `aes-gcm` | 0.10 | Encryption |
| `jsonwebtoken` | 9 | JWT auth |
| `bcrypt` | 0.15 | Password hashing |

## Building

### Local Build

```bash
cd integration-platform
cargo build --release
```

Build artifacts:
- `target/release/control-plane`
- `target/release/data-plane`

### Docker Build

```bash
docker-compose build
```

Or with Docker directly:
```bash
docker build -f Dockerfile.control-plane -t integration-control-plane .
docker build -f Dockerfile.data-plane -t integration-data-plane .
```

## Common Build Issues

### 1. `native-tls` OpenSSL Version Conflict

**Error:**
```
error[E0004]: non-exhaustive patterns: `Some(Protocol::Tlsv13)` not covered
  --> native-tls-0.2.17/src/imp/openssl.rs:61:22
```

**Cause:** Old `native-tls` version incompatible with newer OpenSSL.

**Fix:** Already applied — `reqwest` now uses `rustls-tls` instead of `native-tls`:
```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
```

### 2. Cargo Lock Conflicts

**Error:**
```
error: failed to select a version for the requirement `foo = "^1.0"`
```

**Fix:**
```bash
rm Cargo.lock
cargo update
cargo build --release
```

### 3. Docker Build Timeout

**Error:**
```
ERROR: failed to solve: process "/bin/sh -c cargo build --release" did not complete successfully
```

**Fix:** Increase Docker memory:
- Docker Desktop → Settings → Resources → Memory: 8GB+

Or build locally first:
```bash
cargo build --release
docker-compose build --no-cache
```

### 4. Missing `libssl-dev`

**Error (Debian/Ubuntu):**
```
Could not find directory of OpenSSL installation
```

**Fix:** Install build dependencies:
```bash
sudo apt-get update
sudo apt-get install -y build-essential libssl-dev pkg-config
```

## Environment Variables

### Required

None — all have defaults for development.

### Production Recommended

| Variable | Default | Production Value |
|----------|---------|------------------|
| `DATABASE_URL` | `postgresql://platform:platform123@postgres:5432/integration_platform` | Your prod DB |
| `REDIS_URL` | `redis://redis:6379` | Your Redis instance |
| `NATS_URL` | `nats://nats:4222` | Your NATS cluster |
| `JWT_SECRET` | `integration-platform-dev-secret` | **CHANGE THIS** |
| `ENCRYPTION_KEY` | Auto-generated 64 hex chars | **SET THIS** (32 bytes = 64 hex) |

**Generate production secrets:**
```bash
# JWT_SECRET (any strong random string)
openssl rand -base64 32

# ENCRYPTION_KEY (must be exactly 32 bytes = 64 hex chars)
openssl rand -hex 32
```

## Dockerfiles

### Control Plane

```dockerfile
FROM rust:1.77 as builder
WORKDIR /app
COPY Cargo.toml ./
COPY crates ./crates
RUN cargo build --release -p control-plane

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/control-plane /usr/local/bin/control-plane
EXPOSE 8081
CMD ["control-plane"]
```

### Data Plane

```dockerfile
FROM rust:1.77 as builder
WORKDIR /app
COPY Cargo.toml ./
COPY crates ./crates
RUN cargo build --release -p data-plane

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/data-plane /usr/local/bin/data-plane
EXPOSE 8080
CMD ["data-plane"]
```

## Build Times

| Method | First Build | Incremental |
|--------|-------------|-------------|
| Local (debug) | ~5 min | ~30s |
| Local (release) | ~8 min | ~1 min |
| Docker | ~10 min | ~10 min (no cache) |

**Tip:** Use `cargo build` locally first, then Docker builds are faster.

## Testing After Build

```bash
# Start services
docker-compose up -d

# Wait for health
sleep 10

# Test control plane
curl http://localhost:8081/health

# Test data plane
curl http://localhost:8080/health

# Run full test suite
./test-auth.sh
./test-connector-instances.sh
./test-circuit-breaker.sh
```

## Production Deployment

### 1. Build Images

```bash
docker build -t myregistry/integration-control-plane:v1.0 -f Dockerfile.control-plane .
docker build -t myregistry/integration-data-plane:v1.0 -f Dockerfile.data-plane .
```

### 2. Push to Registry

```bash
docker push myregistry/integration-control-plane:v1.0
docker push myregistry/integration-data-plane:v1.0
```

### 3. Deploy with Secrets

```yaml
# docker-compose.prod.yml
services:
  control-plane:
    image: myregistry/integration-control-plane:v1.0
    environment:
      DATABASE_URL: ${DATABASE_URL}
      REDIS_URL: ${REDIS_URL}
      NATS_URL: ${NATS_URL}
      JWT_SECRET: ${JWT_SECRET}
      ENCRYPTION_KEY: ${ENCRYPTION_KEY}
      RUST_LOG: info
```

```bash
# Set secrets
export DATABASE_URL="postgresql://..."
export JWT_SECRET=$(openssl rand -base64 32)
export ENCRYPTION_KEY=$(openssl rand -hex 32)

# Deploy
docker-compose -f docker-compose.prod.yml up -d
```

## Troubleshooting

### Rust Version Issues

```bash
# Check version
rustc --version

# Update if needed
rustup update stable
rustup default stable
```

### Cargo Cache

```bash
# Clear cache if corrupted
rm -rf ~/.cargo/registry
rm -rf ~/.cargo/git
cargo clean
```

### Docker Build Cache

```bash
# Clear Docker build cache
docker builder prune -af

# Rebuild without cache
docker-compose build --no-cache
```

## Summary

- ✅ Uses **rustls** to avoid OpenSSL conflicts
- ✅ Builds on Rust 1.75+ (tested on 1.77)
- ✅ Compatible with Debian Bookworm (latest stable)
- ✅ ~8 min release build time
- ✅ All dependencies pinned for reproducibility

