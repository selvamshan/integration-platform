# Keycloak Troubleshooting

Common Keycloak setup issues and solutions.

---

## Issue: "database 'keycloak' does not exist"

**Error:**
```
ERROR: Failed to obtain JDBC connection
ERROR: FATAL: database "keycloak" does not exist
```

**Cause:**  
Keycloak is configured to use a database that doesn't exist in PostgreSQL.

**Solution:**  
Keycloak now shares the `integration_platform` database with the Control Plane. This is already configured in `docker-compose.yml`:

```yaml
keycloak:
  environment:
    KC_DB: postgres
    KC_DB_URL: jdbc:postgresql://postgres:5432/integration_platform
    KC_DB_USERNAME: platform
    KC_DB_PASSWORD: platform123
```

**Restart to apply:**
```bash
docker-compose down
docker-compose up -d
```

Keycloak will create its tables in the same database (different table names, no conflicts).

---

## Issue: Keycloak takes long to start

**Symptom:**  
Keycloak container shows as "starting" for 1-2 minutes.

**Cause:**  
Keycloak needs to:
1. Connect to PostgreSQL
2. Run database migrations
3. Initialize Quarkus runtime

**Solution:**  
This is normal! Wait for:
```bash
docker-compose logs -f keycloak | grep "Running"
```

When you see:
```
Listening on: http://0.0.0.0:8080
```

Keycloak is ready.

---

## Issue: Port 8080 already in use

**Error:**
```
Error starting userland proxy: listen tcp4 0.0.0.0:8080: bind: address already in use
```

**Cause:**  
Data Plane also uses port 8080.

**Solution:**  
Keycloak is mapped to **port 8180** (not 8080) to avoid conflicts:

```yaml
keycloak:
  ports:
    - "8180:8080"  # Host 8180 → Container 8080
```

**Access Keycloak at:**
- Admin Console: http://localhost:8180/auth/admin
- Realm URL: http://localhost:8180/auth/realms/integration-platform

---

## Issue: Cannot access Keycloak admin console

**Symptom:**  
http://localhost:8180/auth/admin returns 404 or connection refused.

**Check:**

1. **Container running?**
   ```bash
   docker-compose ps keycloak
   # Should show "Up"
   ```

2. **Check logs:**
   ```bash
   docker-compose logs keycloak | tail -50
   ```

3. **Try alternate URL:**
   - Old Keycloak: http://localhost:8180/auth/admin
   - New Keycloak (23+): http://localhost:8180/admin

4. **Verify port mapping:**
   ```bash
   docker-compose ps | grep keycloak
   # Should show: 0.0.0.0:8180->8080/tcp
   ```

---

## Issue: Admin login fails

**Default credentials:**
- Username: `admin`
- Password: `admin123`

If these don't work:

1. **Check environment variables:**
   ```bash
   docker-compose exec keycloak env | grep KEYCLOAK_ADMIN
   # Should show:
   # KEYCLOAK_ADMIN=admin
   # KEYCLOAK_ADMIN_PASSWORD=admin123
   ```

2. **Recreate container:**
   ```bash
   docker-compose down keycloak
   docker volume rm integration-platform_keycloak_data  # if it exists
   docker-compose up -d keycloak
   ```

---

## Issue: Realm not found

**Error when getting token:**
```
{"error":"Realm does not exist"}
```

**Solution:**  
You need to create the realm first:

1. Go to: http://localhost:8180/auth/admin
2. Login: admin / admin123
3. Click "Create realm"
4. Name: `integration-platform`
5. Save

Then retry token request.

---

## Issue: Client not found

**Error:**
```
{"error":"Client not found"}
```

**Solution:**  
Create the client in Keycloak:

1. Realm: `integration-platform`
2. Clients → Create client
3. Client ID: `control-plane`
4. Client authentication: **ON**
5. Save
6. Copy client secret from Credentials tab

---

## Issue: Token validation fails in Control Plane

**Error in control-plane logs:**
```
Token validation failed: Failed to fetch Keycloak public key
```

**Check:**

1. **Keycloak URL correct?**
   ```bash
   docker-compose exec control-plane env | grep KEYCLOAK_SERVER_URL
   # Should be: http://keycloak:8080
   # Note: Internal URL uses port 8080, not 8180
   ```

2. **Test connectivity:**
   ```bash
   docker-compose exec control-plane curl http://keycloak:8080/auth/realms/integration-platform/.well-known/openid-configuration
   # Should return JSON config
   ```

3. **Restart control-plane:**
   ```bash
   docker-compose restart control-plane
   ```

---

## Issue: Tables conflict between Keycloak and Control Plane

**Concern:**  
Both use the same database - will tables conflict?

**Answer:**  
No conflicts! Keycloak uses specific table prefixes:
- Keycloak tables: `ADMIN_EVENT_ENTITY`, `AUTHENTICATION_FLOW`, `CLIENT`, etc.
- Control Plane tables: `users`, `flows`, `apis`, `connector_instances`, etc.

They coexist peacefully in the same database.

---

## Clean Slate Reset

To completely reset Keycloak:

```bash
# Stop everything
docker-compose down

# Remove Keycloak data (this deletes all Keycloak users/realms)
docker-compose exec postgres psql -U platform -d integration_platform -c "
  DROP TABLE IF EXISTS ADMIN_EVENT_ENTITY CASCADE;
  DROP TABLE IF EXISTS AUTHENTICATION_EXECUTION CASCADE;
  DROP TABLE IF EXISTS AUTHENTICATION_FLOW CASCADE;
  DROP TABLE IF EXISTS AUTHENTICATOR_CONFIG CASCADE;
  DROP TABLE IF EXISTS CLIENT CASCADE;
  DROP TABLE IF EXISTS CLIENT_ATTRIBUTES CASCADE;
  DROP TABLE IF EXISTS CLIENT_SESSION CASCADE;
  -- ... and all other Keycloak tables
"

# Or easier: drop and recreate entire database
docker-compose down
docker volume rm integration-platform_postgres_data
docker-compose up -d postgres

# Wait for postgres
sleep 10

# Start everything
docker-compose up -d
```

**Warning:** This also deletes Control Plane data (flows, connectors, etc).

---

## Verification Checklist

After setup, verify:

```bash
# 1. Keycloak is running
docker-compose ps keycloak
# Status: Up

# 2. Admin console accessible
curl -I http://localhost:8180/admin
# Should return 200 OK

# 3. Realm exists
curl http://localhost:8180/realms/integration-platform/.well-known/openid-configuration
# Should return JSON

# 4. Can get token
curl -X POST http://localhost:8180/realms/integration-platform/protocol/openid-connect/token \
  -d "client_id=control-plane" \
  -d "client_secret=YOUR_SECRET" \
  -d "username=admin-user" \
  -d "password=admin123" \
  -d "grant_type=password"
# Should return access_token

# 5. Control Plane can validate
curl http://localhost:8081/users/me -H "Authorization: Bearer YOUR_TOKEN"
# Should return user info
```

---

## Production Recommendations

1. **Separate Database**  
   In production, use a dedicated database for Keycloak:
   ```yaml
   KC_DB_URL: jdbc:postgresql://keycloak-db:5432/keycloak
   ```

2. **External PostgreSQL**  
   Don't use Docker PostgreSQL in production:
   ```yaml
   KC_DB_URL: jdbc:postgresql://prod-db.aws.com:5432/keycloak
   ```

3. **HTTPS Only**  
   Never run Keycloak on HTTP in production:
   ```yaml
   KC_HOSTNAME: auth.example.com
   KC_HTTPS_CERTIFICATE_FILE: /cert/fullchain.pem
   KC_HTTPS_CERTIFICATE_KEY_FILE: /cert/privkey.pem
   ```

4. **Strong Admin Password**  
   ```yaml
   KEYCLOAK_ADMIN_PASSWORD: <strong-random-password>
   ```

---

## Summary

| Issue | Fix |
|-------|-----|
| Database doesn't exist | Uses `integration_platform` (shared) |
| Port conflict | Keycloak on 8180, Data Plane on 8080 |
| Admin console 404 | Try `/admin` or `/auth/admin` |
| Token validation fails | Check `KEYCLOAK_SERVER_URL` in control-plane |
| Long startup time | Normal - wait 1-2 minutes |

**For dev/testing:** Current setup works great (shared database, dev mode).  
**For production:** Use separate database, HTTPS, strong passwords.
