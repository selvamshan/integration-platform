#!/bin/bash
set -e

echo "Testing local build..."
cd crates/common && cargo check 2>&1 | head -20
cd ../integration-runtime && cargo check 2>&1 | head -20
cd ../data-plane && cargo check 2>&1 | head -20
cd ../control-plane && cargo check 2>&1 | head -20
