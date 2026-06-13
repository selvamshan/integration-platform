# Deployment

## Docker Compose (Development / Small Production)

```bash
docker-compose up --build -d
```

Starts all services: Control Plane, Data Plane, PostgreSQL, NATS, Redis, Keycloak.

## Building Docker Images

```bash
# Control Plane
docker build -f crates/control-plane/Dockerfile -t integration-control-plane:latest .

# Data Plane
docker build -f crates/data-plane/Dockerfile -t integration-data-plane:latest .
```

## Environment Variables (Production)

Set these as secrets in your deployment platform (AWS Secrets Manager, Kubernetes Secrets, etc.):

```env
DATABASE_URL=postgres://user:pass@db.internal:5432/integration
NATS_URL=nats://nats.internal:4222
REDIS_URL=redis://redis.internal:6379
KEYCLOAK_URL=https://auth.example.com
KEYCLOAK_REALM=integration-platform
KEYCLOAK_CLIENT_ID=control-plane
KEYCLOAK_CLIENT_SECRET=<secret>
JWT_SECRET=<64-char random string>
ENCRYPTION_KEY=<32-byte hex key>
RUST_LOG=info
```

## Kubernetes

Minimal deployment sketch:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: data-plane
spec:
  replicas: 3
  selector:
    matchLabels: { app: data-plane }
  template:
    metadata:
      labels: { app: data-plane }
    spec:
      containers:
        - name: data-plane
          image: integration-data-plane:latest
          ports: [{ containerPort: 8080 }]
          envFrom:
            - secretRef: { name: integration-platform-secrets }
          readinessProbe:
            httpGet: { path: /health, port: 8080 }
          livenessProbe:
            httpGet: { path: /health, port: 8080 }
```

## AWS ECS / Fargate

Use the official AWS CDK or Terraform to define ECS task definitions. Key configuration:

- Run Control Plane and Data Plane as separate ECS services.
- Use RDS PostgreSQL for the database.
- Use ElastiCache Redis for rate limiting.
- Use Amazon MQ (NATS) or MSK for the event bus.
- Use AWS Secrets Manager for all secrets (mounted as environment variables).

## Health Checks

| Service | Endpoint | Expected |
|---------|----------|---------|
| Control Plane | `GET /health` | `{"status":"healthy"}` |
| Data Plane | `GET /health` | `{"status":"healthy"}` |

Configure load balancer health checks to these endpoints.

## Scaling

- **Data Plane** scales horizontally — add more replicas behind a load balancer. Each instance independently caches flows via NATS.
- **Control Plane** is stateless (state in PostgreSQL) and can also scale horizontally.
- Use a PostgreSQL connection pooler (PgBouncer) when scaling beyond ~10 replicas.
