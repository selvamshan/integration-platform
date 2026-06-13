# Audit Logs

The Control Plane writes a tamper-evident audit record for every mutating action.

## What is Logged

| Action | Recorded fields |
|--------|----------------|
| Create / Update / Delete flow | actor, flow ID, before/after payload |
| Create / Delete connector | actor, connector ID |
| User invite / delete | actor, target user, role |
| Login / token issue | actor, IP address |

## Audit Log Entry

```json
{
  "id": "uuid",
  "timestamp": "2024-02-09T12:34:56Z",
  "actor_id": "user-uuid",
  "actor_email": "admin@example.com",
  "action": "flow.create",
  "resource_type": "flow",
  "resource_id": "my-flow-id",
  "payload": { ... },
  "previous_hash": "sha256:...",
  "hash": "sha256:..."
}
```

Each entry includes the hash of the previous entry, forming a chain. Any tampering with a past record invalidates all subsequent hashes.

## Query Audit Logs

```bash
# All logs (admin only)
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8081/audit-logs?limit=50"

# Filter by resource
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8081/audit-logs?resource_type=flow&resource_id=my-flow"

# Filter by actor
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8081/audit-logs?actor_id=user-uuid"
```

## Verify Chain Integrity

```bash
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8081/audit-logs/verify"
```

Returns `{ "valid": true }` if the hash chain is intact, or details of the first broken link.

## Retention

Audit logs are stored in PostgreSQL in the `audit_logs` table. Configure retention with a cron job or PostgreSQL partitioning — no built-in TTL is applied by the platform.
