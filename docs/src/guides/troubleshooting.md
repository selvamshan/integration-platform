# Troubleshooting

## Build Fails with Exit Code 101

Docker ran out of memory during the Rust compile.

**Fix:** Increase Docker memory to at least 4 GB:
- Docker Desktop → Settings → Resources → Memory → 4 GB+

Or build on the host instead of in Docker:
```bash
cargo build --release
```

## Services Fail Health Check

```bash
# Check container logs
docker-compose logs control-plane
docker-compose logs data-plane

# Check if PostgreSQL is ready
docker-compose exec postgres pg_isready -U postgres
```

Common cause: PostgreSQL is still starting when the application tries to connect. Wait 10–15 seconds and retry.

## 401 Unauthorized from Control Plane

1. Confirm your token is not expired: `jwt decode $TOKEN`
2. Confirm Keycloak URL and realm are correct in the Control Plane env
3. Confirm the token audience (`aud`) matches `KEYCLOAK_CLIENT_ID`

## 403 Forbidden

Your user account has insufficient role for the action. Check the [RBAC guide](./rbac.md) for required permissions.

## 429 Too Many Requests

You have exceeded a rate limit. Check the `Retry-After` header and wait before retrying. See [Rate Limiting](./rate-limiting.md) to adjust limits.

## Flows Not Updating After Edit

The Data Plane caches flows in memory and refreshes via NATS. Check:

```bash
# Is NATS running?
docker-compose ps nats

# NATS logs
docker-compose logs nats
```

If NATS is down, restart it and the Data Plane will re-warm its cache from the Control Plane REST API.

## Database Migration Errors

```bash
# Run migrations manually
sqlx migrate run \
  --source crates/control-plane/migrations \
  --database-url $DATABASE_URL
```

Check that `DATABASE_URL` points to the correct database and the user has DDL privileges.

## Oracle Connector Fails to Load

```
error: ORA-12154: TNS:could not resolve the connect identifier specified
```

Ensure Oracle Instant Client is installed and `LD_LIBRARY_PATH` is set in the container or host environment.

## High Memory Usage

The Data Plane holds all flow definitions in memory. If you have thousands of large flows:

- Increase the container memory limit
- Consider partitioning flows across multiple Data Plane instances by project

## Connector Credential Encryption Errors

```
error: decryption failed
```

This happens if `ENCRYPTION_KEY` was changed after credentials were stored. All connector instances must be re-registered with the new key.
