#!/bin/bash
# Script to enable RBAC middleware in control-plane

set -e

echo "════════════════════════════════════════════════════════════════"
echo "              Enabling RBAC Middleware                          "
echo "════════════════════════════════════════════════════════════════"
echo ""

# Check if source file exists
if [ ! -f "crates/control-plane/src/main.rs" ]; then
    echo "❌ Error: crates/control-plane/src/main.rs not found"
    echo "   Run this script from the integration-platform directory"
    exit 1
fi

# Create backup
cp crates/control-plane/src/main.rs crates/control-plane/src/main.rs.backup
echo "✅ Created backup: main.rs.backup"

# Uncomment the RBAC middleware lines
sed -i 's|^        // .layer(middleware::from_fn(permission_middleware))|        .layer(middleware::from_fn(permission_middleware))|' crates/control-plane/src/main.rs
sed -i 's|^        // .layer(middleware::from_fn_with_state(keycloak.clone(), rbac_middleware))|        .layer(middleware::from_fn_with_state(keycloak.clone(), rbac_middleware))|' crates/control-plane/src/main.rs

echo "✅ Uncommented RBAC middleware lines"

# Verify changes
if grep -q "^        .layer(middleware::from_fn(permission_middleware))" crates/control-plane/src/main.rs && \
   grep -q "^        .layer(middleware::from_fn_with_state(keycloak.clone(), rbac_middleware))" crates/control-plane/src/main.rs; then
    echo "✅ RBAC middleware is now ENABLED"
    echo ""
    echo "Next steps:"
    echo "──────────────────────────────────────────────────────────────────"
    echo "1. Set KEYCLOAK_CLIENT_SECRET (if not already set):"
    echo "   export KEYCLOAK_CLIENT_SECRET='your-secret-from-keycloak'"
    echo ""
    echo "2. Rebuild control-plane:"
    echo "   docker-compose build control-plane"
    echo ""
    echo "3. Restart control-plane:"
    echo "   docker-compose restart control-plane"
    echo ""
    echo "4. Test with your token:"
    echo "   curl http://localhost:8081/users/me -H \"Authorization: Bearer \$TOKEN\""
    echo ""
else
    echo "⚠️  Warning: Could not verify changes"
    echo "   Please manually uncomment these lines in crates/control-plane/src/main.rs:"
    echo "   // .layer(middleware::from_fn(permission_middleware))"
    echo "   // .layer(middleware::from_fn_with_state(keycloak.clone(), rbac_middleware))"
fi
