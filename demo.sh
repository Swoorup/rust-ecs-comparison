#!/bin/bash

# Comprehensive demo script for Flax ECS REPL
# Demonstrates all major features in a coherent RPG scenario
echo "🎮 Running comprehensive Flax ECS REPL demo..."
echo "This demo showcases entities, relationships, components, magic system, and queries"
echo ""

cat << 'EOF' | cargo run --bin rust-ecs-comparison
# === RPG World Setup ===
echo "🏰 Creating the RPG world..."

# Create the royal family
add entity king
add entity queen 
add entity prince
add entity princess

# Create adventurers and NPCs
add entity wizard
add entity warrior
add entity rogue
add entity merchant
add entity dragon

echo "👑 Setting up the royal family hierarchy..."
set-relation child prince parent king
set-relation child princess parent king
set-relation child prince parent queen
set-relation child princess parent queen

echo "⚔️ Initializing character stats..."
# Set health for all characters
set health king 120
set health queen 100
set health prince 80
set health princess 75
set health wizard 60
set health warrior 150
set health rogue 90
set health merchant 50
set health dragon 300

echo "🔮 Distributing magical powers..."
# Give mana to magical beings
set mana king 50
set mana queen 80
set mana wizard 200
set mana princess 40
set mana dragon 500

echo "📊 Showing initial world state..."
dump

echo ""
echo "🎯 Demonstrating tree traversals..."
tree dfs
tree topo

echo ""
echo "⚡ Battle simulation begins..."
# Wizard attacks
cast fireball wizard 50
cast lightning wizard 30

# Princess heals
cast heal princess 20

# Dragon retaliates
cast fireball dragon 100

echo ""
echo "🏥 Checking health and mana after battle..."
get wizard
get princess
get dragon

echo ""
echo "🔍 Showing only modified entities..."
dump modified

echo ""
echo "💀 Tragic events unfold..."
# Remove some entities to show Drop implementation
rm merchant
rm rogue

echo ""
echo "👥 Final kingdom status..."
list

echo ""
echo "🏴‍☠️ Checking for orphaned entities..."
dump

echo ""
echo "📈 Complete world overview..."
tree

quit
EOF

echo ""
echo "✨ Demo completed! This showcased:"
echo "• Entity creation and management"
echo "• Parent-child relationships" 
echo "• Health and mana components"
echo "• Spell casting with mana consumption"
echo "• Component Drop implementation"
echo "• Change tracking systems"
echo "• Tree traversal algorithms (DFS/Topo)"
echo "• Orphaned entity detection"
echo "• Various query types and filters"