#!/bin/bash

# Test script for system-based change detection
echo "Testing system-based change detection..."

# Create a test input sequence
cat << 'EOF' | cargo run --bin rust-ecs-comparison
add entity player1
set health player1 100
add entity enemy1
set health enemy1 50
dump added
dump modified
quit
EOF