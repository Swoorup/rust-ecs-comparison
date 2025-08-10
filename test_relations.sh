#!/bin/bash

# Test script for system-based change detection
echo "Testing system-based change detection..."

# Create a test input sequence
cat << 'EOF' | cargo run --bin rust-ecs-comparison
add entity charlie
add entity margaret

add entity bob
add entity alice

set-relation child charlie parent bob
set-relation child charlie parent alice
set-relation child margaret parent bob
set-relation child margaret parent alice
dump
quit
EOF
