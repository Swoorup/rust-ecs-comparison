#!/bin/bash

# Test script for demonstrating entities without relationships
echo "Testing without_relation query functionality..."

cat << 'EOF' | cargo run --bin rust-ecs-comparison
add entity alice
add entity bob
add entity charlie
add entity diana
add entity eve

# Create some relationships
set-relation child alice parent bob
set-relation child bob parent charlie

# diana and eve should show as orphaned entities
dump

quit
EOF