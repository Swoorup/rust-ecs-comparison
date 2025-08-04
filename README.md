# ECS Library Comparison Project

A comprehensive comparison of 6 different Entity Component System (ECS) libraries in Rust, demonstrating production-ready patterns with type-safe entity handles and command systems.

## Overview

This project implements the same pane-dataset management system across multiple ECS libraries, each enhanced with production-ready patterns to provide a realistic comparison of real-world usage.

## Featured Libraries

| Library | Approach | Key Features |
|---------|----------|--------------|
| **Flax** | Relations-based | Type-safe handles, modular components, built-in relations, command system |
| **Evenio** | Event-driven | Type-safe handles, command system, registry pattern, event architecture |
| **Hecs + Hierarchy** | Hierarchy-based | Type-safe handles, command system, parent-child relationships |
| **Bevy ECS** | Component-based | Type-safe handles, command system, modern API, wrapper relationships |
| **Sparsey** | Group-based | Type-safe handles, component groups |
| **Flecs** | Limited Rust API | Type-safe handles, simulated command system |

## Running Examples

Each library implementation is available as a separate binary:

```bash
# Run individual examples
cargo run --bin flax_example
cargo run --bin evenio_example  
cargo run --bin hecs_example
cargo run --bin bevy_ecs_example
cargo run --bin sparsey_example
cargo run --bin flecs_example
```

## Production Patterns Demonstrated

All examples showcase production-ready patterns:

- **Type-Safe Entity Handles**: Compile-time prevention of entity type mixing
- **Command Systems**: Queue-based deferred execution for safer entity lifecycle management
- **Realistic Data**: Sensor data examples instead of artificial test data
- **Comprehensive Functionality**: Beyond basic component storage and queries

## Detailed Analysis

See [ECS_COMPARISON.md](ECS_COMPARISON.md) for an in-depth analysis of each library including:

- Code examples with production patterns
- Performance characteristics
- Readability and maintainability assessment
- Production-readiness evaluation
- Detailed recommendations for different use cases

## Key Findings

The comparison reveals that **Flax Enhanced** provides the best combination of type safety, semantic relations, modular organization, and zero-cost abstractions, making it the most suitable choice for large-scale production applications.

## Requirements

- Rust 2024 edition
- See `Cargo.toml` for specific dependency versions

## License

This project is for educational and comparison purposes, demonstrating different approaches to ECS architecture in Rust.