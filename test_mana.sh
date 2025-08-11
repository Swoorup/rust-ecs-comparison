#!/bin/bash

# Test script for the Mana component with Drop implementation
echo "Testing Mana component with Drop implementation..."

# Create a test input sequence demonstrating mana and spell casting
cat << 'EOF' | cargo run --bin rust-ecs-comparison
add entity wizard
add entity sorcerer
add entity apprentice

set mana wizard 100
set mana sorcerer 50
set mana apprentice 20

get wizard
get sorcerer
get apprentice

cast fireball wizard 30
cast lightning sorcerer 20
cast heal apprentice 15

get wizard
get sorcerer
get apprentice

cast teleport wizard 70
cast shield sorcerer 30
cast fireball apprentice 10

remove wizard
remove sorcerer
remove apprentice

quit
EOF