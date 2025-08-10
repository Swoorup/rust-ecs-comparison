#!/bin/bash

# Test script for more complex change detection scenarios
echo "Testing system-based change detection with modifications..."

# Create a test input sequence with modifications
cat << 'EOF' | cargo run --bin rust-ecs-comparison
add entity warrior
set health warrior 75
dump added
set health warrior 25
dump modified
add entity mage
dump
quit
EOF