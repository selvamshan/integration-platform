# Build Troubleshooting Guide

## Common Build Issues and Solutions

### Issue 1: Docker Build Fails with Exit Code 101

**Symptom:**
```
failed to solve: process "/bin/sh -c cargo build --release -p data-plane" did not complete successfully: exit code: 101
```

**Solutions:**

#### Solution A: Increase Docker Memory
Rust compilation requires significant memory. Increase Docker's memory allocation:

**Docker Desktop:**
1. Open Docker Desktop
2. Go to Settings → Resources
3. Increase Memory to at least 4GB (8GB recommended)
4. Click "Apply & Restart"

**Docker on Linux:**
```bash
# Check current memory
docker info | grep Memory

# No limit needed on Linux, but ensure host has enough RAM
```

#### Solution B: Use Pre-built Binaries
Build locally instead of in Docker:

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build locally
cd integration-platform
cargo build --release

# Then use local binaries
cp target/release/data-plane ./data-plane-binary
cp target/release/control-plane ./control-plane-binary
```

Update docker-compose.yml to use binaries:
```yaml
# Add this to docker-compose.yml for data-plane
data-plane:
  image: debian:bookworm-slim
  volumes:
    - ./data-plane-binary:/usr/local/bin/data-plane
  # ... rest of config
```

#### Solution C: Use Smaller Build (Debug Mode)
Edit Dockerfiles to use debug builds (faster, larger):

```dockerfile
# Change this line in both Dockerfiles
RUN cargo build -p data-plane  # Remove --release
```

#### Solution D: Build with More Time
Add build timeout to docker-compose:

```yaml
services:
  data-plane:
    build:
      context: .
      dockerfile: Dockerfile.data-plane
      args:
        BUILDKIT_INLINE_CACHE: 1
    # ... rest
```

Then build with:
```bash
DOCKER_BUILDKIT=1 docker-compose build --progress=plain
```

### Issue 2: PostgreSQL Connection Errors

**Symptom:**
```
error: error returned from database: could not connect to server
```

**Solutions:**

```bash
# Wait for PostgreSQL to be ready
docker-compose up -d postgres
sleep 10
docker-compose up data-plane control-plane

# Or check PostgreSQL logs
docker-compose logs postgres

# Restart if needed
docker-compose restart postgres
```

### Issue 3: Port Already in Use

**Symptom:**
```
Error: Address already in use (os error 48)
```

**Solutions:**

```bash
# Find what's using the port
lsof -i :8080  # For data-plane
lsof -i :8081  # For control-plane
lsof -i :5432  # For PostgreSQL

# Kill the process or change ports in docker-compose.yml
ports:
  - "8082:8080"  # Use different host port
```

### Issue 4: Dependency Download Fails

**Symptom:**
```
error: failed to download from `https://crates.io/...`
```

**Solutions:**

```bash
# Use a different mirror
export CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

# Or retry with clean cache
docker-compose build --no-cache

# Or use vendored dependencies (offline mode)
cargo vendor
```

### Issue 5: Out of Disk Space

**Symptom:**
```
no space left on device
```

**Solutions:**

```bash
# Clean Docker
docker system prune -a --volumes

# Clean Rust cache
cargo clean

# Check disk space
df -h
```

## Quick Fixes

### Quick Fix 1: Clean Slate
```bash
# Stop everything
docker-compose down -v

# Clean Docker
docker system prune -a

# Rebuild
docker-compose up --build
```

### Quick Fix 2: Build Only One Service
```bash
# Build just data-plane
docker-compose build data-plane

# Then start everything
docker-compose up
```

### Quick Fix 3: Use Host Network (Linux only)
```yaml
# In docker-compose.yml
services:
  data-plane:
    network_mode: host
    # Change ports to actual ports (no mapping)
```

## Alternative: Build Outside Docker

If Docker builds continue to fail, build on your host:

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 2. Install PostgreSQL dev libraries
# Ubuntu/Debian:
sudo apt-get install libpq-dev

# macOS:
brew install postgresql

# 3. Build
cd integration-platform
cargo build --release

# 4. Start PostgreSQL only
docker-compose up -d postgres

# 5. Set database URL
export DATABASE_URL=postgresql://platform:platform123@localhost:5432/integration_platform

# 6. Run services
cargo run --release -p control-plane &
cargo run --release -p data-plane &
```

## Environment-Specific Issues

### Windows with WSL2
```bash
# Ensure WSL2 has enough memory
# Create/edit ~/.wslconfig:
[wsl2]
memory=8GB
processors=4
```

### macOS with ARM (M1/M2)
```bash
# Use ARM-compatible base images
# Change in Dockerfiles:
FROM --platform=linux/arm64 rust:1.75 as builder
```

### Linux with SELinux
```bash
# Disable SELinux for Docker
sudo setenforce 0

# Or add :z to volumes
volumes:
  - ./data:/data:z
```

## Still Having Issues?

### Enable Verbose Logging
```bash
# Build with full output
DOCKER_BUILDKIT=0 docker-compose build 2>&1 | tee build.log

# Check the log
less build.log
```

### Check Individual Crate Compilation
```bash
cd crates/common
cargo check --verbose

cd ../integration-runtime
cargo check --verbose

cd ../data-plane
cargo check --verbose
```

### Use Pre-built Images (Coming Soon)
```yaml
# In docker-compose.yml (when available)
services:
  data-plane:
    image: integration-platform/data-plane:latest
    # No build needed
```

## Contact & Support

If none of these solutions work:

1. Check Rust version: `rustc --version` (should be 1.75+)
2. Check Docker version: `docker --version` (should be 20.10+)
3. Check available memory: `free -h` or `vm_stat`
4. Share build logs for diagnosis

## Success Checklist

Before building, ensure:
- [ ] Docker has 4GB+ memory allocated
- [ ] At least 10GB free disk space
- [ ] Internet connection is stable
- [ ] No other services on ports 8080, 8081, 5432
- [ ] Docker daemon is running
- [ ] You're in the correct directory

## Quick Test After Build

```bash
# Test services started
docker-compose ps

# Test health endpoints
curl http://localhost:8080/health
curl http://localhost:8081/health

# Test database
docker-compose exec postgres psql -U platform -d integration_platform -c "SELECT 1"

# Test actual flow
curl http://localhost:8080/api/trigger/users
```

Good luck! 🚀
