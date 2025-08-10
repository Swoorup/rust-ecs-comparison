#![allow(unused)]
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::Schedule;
use std::collections::{HashMap, VecDeque};

// Macro to create type-safe entity handles
macro_rules! entity_handles {
    ($($handle_name:ident),* $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub struct $handle_name(Entity);

            impl $handle_name {
                pub fn new(entity: Entity) -> Self {
                    Self(entity)
                }

                pub fn entity(&self) -> Entity {
                    self.0
                }
            }

            impl From<Entity> for $handle_name {
                fn from(entity: Entity) -> Self {
                    Self(entity)
                }
            }

            impl From<$handle_name> for Entity {
                fn from(handle: $handle_name) -> Self {
                    handle.0
                }
            }
        )*
    };
}

// Create type-safe handles
entity_handles! {
    PaneHandle,
    DatasetHandle,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DatasetId(&'static str);

#[derive(Component, Debug, Clone)]
struct Pane {
    width: u32,
    height: u32,
}

// Relationship components - Bevy ECS built-in relationships
#[derive(Component, Debug, Clone)]
#[relationship(relationship_target = DatasetSubscribers)]
struct UsesDataset {
    #[relationship]
    dataset: Entity,
}

#[derive(Component, Debug, Clone)]
#[relationship_target(relationship = UsesDataset)]
struct DatasetSubscribers(Vec<Entity>);

// Command system resources - global state
#[derive(Resource, Debug, Clone)]
struct CommandQueue {
    commands: VecDeque<Command>,
}

#[derive(Resource, Debug, Clone)]
struct CreatedPanes {
    panes: Vec<(Vec<DatasetId>, PaneHandle)>,
}

// Command types
#[derive(Debug, Clone)]
pub enum Command {
    CreatePaneWithDatasets { dataset_ids: Vec<DatasetId> },
    DeletePane { pane: PaneHandle },
}

// System-compatible pane creation
fn create_pane_with_datasets_system(
    commands: &mut Commands,
    dataset_ids: Vec<DatasetId>,
    datasets_query: &Query<(Entity, &DatasetId)>,
) -> PaneHandle {
    // Create the pane entity
    let pane = commands
        .spawn(Pane {
            width: 100,
            height: 200,
        })
        .id();
    let pane_handle = PaneHandle::new(pane);

    for dataset_id in dataset_ids {
        // Find existing dataset by querying all datasets
        let mut existing_dataset = None;
        for (entity, id) in datasets_query.iter() {
            if *id == dataset_id {
                existing_dataset = Some(DatasetHandle::new(entity));
                break;
            }
        }

        let dataset_handle = if let Some(existing) = existing_dataset {
            existing
        } else {
            // Create new dataset entity
            let dataset_entity = commands.spawn(dataset_id).id();
            DatasetHandle::new(dataset_entity)
        };

        // Create the relationships using Bevy's relationship system
        commands.entity(pane).insert(UsesDataset {
            dataset: dataset_handle.entity(),
        });
    }

    pane_handle
}

// Legacy function for non-system usage
fn create_pane_with_datasets(world: &mut World, dataset_ids: Vec<DatasetId>) -> PaneHandle {
    // Create the pane entity
    let pane = world
        .spawn(Pane {
            width: 100,
            height: 200,
        })
        .id();
    let pane_handle = PaneHandle::new(pane);

    for dataset_id in dataset_ids {
        // Find existing dataset by querying all datasets
        let mut existing_dataset = None;
        for (entity, id) in world.query::<(Entity, &DatasetId)>().iter(world) {
            if *id == dataset_id {
                existing_dataset = Some(DatasetHandle::new(entity));
                break;
            }
        }

        let dataset_handle = if let Some(existing) = existing_dataset {
            existing
        } else {
            // Create new dataset entity
            let dataset_entity = world.spawn(dataset_id).id();
            DatasetHandle::new(dataset_entity)
        };

        // Create the relationships using Bevy's relationship system
        world.entity_mut(pane).insert(UsesDataset {
            dataset: dataset_handle.entity(),
        });
    }

    pane_handle
}

fn get_panes_for_dataset(world: &World, dataset: DatasetHandle) -> Vec<PaneHandle> {
    let mut subscribing_panes = Vec::new();

    // Query the relationship target component for this dataset
    if let Ok(entity_ref) = world.get_entity(dataset.entity()) {
        if let Some(subscribers) = entity_ref.get::<DatasetSubscribers>() {
            subscribing_panes.extend(subscribers.0.iter().map(|&e| PaneHandle::new(e)));
        }
    }

    subscribing_panes
}

// Command processing system - proper Bevy system function
fn process_commands_system(
    mut commands: Commands,
    mut command_queue: ResMut<CommandQueue>,
    mut created_panes: ResMut<CreatedPanes>,
    datasets_query: Query<(Entity, &DatasetId)>,
) {
    // Get and process all pending commands
    let pending_commands: Vec<Command> = command_queue.commands.drain(..).collect();

    // Process commands and collect results
    let mut new_panes = Vec::new();
    let mut deleted_panes = Vec::new();

    for cmd in pending_commands {
        match cmd {
            Command::CreatePaneWithDatasets { dataset_ids } => {
                println!(
                    "[System] Processing CreatePaneWithDatasets command with {} datasets",
                    dataset_ids.len()
                );
                let pane_handle = create_pane_with_datasets_system(
                    &mut commands,
                    dataset_ids.clone(),
                    &datasets_query,
                );
                new_panes.push((dataset_ids, pane_handle));
                println!("[System] Created pane: {:?}", pane_handle);
            }
            Command::DeletePane { pane } => {
                println!("[System] Processing DeletePane command for {:?}", pane);
                commands.entity(pane.entity()).despawn();
                deleted_panes.push(pane);
            }
        }
    }

    // Update created_panes tracking after processing
    for new_pane in new_panes {
        created_panes.panes.push(new_pane);
    }
    for deleted_pane in deleted_panes {
        created_panes.panes.retain(|(_, h)| *h != deleted_pane);
    }
}

// Helper to enqueue commands using resources
fn enqueue_command(world: &mut World, cmd: Command) {
    let mut queue = world.resource_mut::<CommandQueue>();
    queue.commands.push_back(cmd);
}

fn dump_subscriptions_by_dataset(world: &mut World) {
    // Print all datasets and their subscriptions
    println!("\n=== Dataset Subscriptions ===");

    for (entity, dataset_id) in world.query::<(Entity, &DatasetId)>().iter(world) {
        println!("Dataset: {:#?}", dataset_id);
        println!("  Handle: {:?}", DatasetHandle::new(entity));

        // Use the dedicated function to get panes for this dataset
        let subscribing_panes = get_panes_for_dataset(&world, DatasetHandle::new(entity));

        if !subscribing_panes.is_empty() {
            println!(
                "  Subscribed by {} panes: {:?}",
                subscribing_panes.len(),
                subscribing_panes
            );
        } else {
            println!("  No pane subscriptions");
        }
    }
}

pub fn main() {
    // Create a new bevy_ecs world
    let mut world = World::new();

    // Initialize resources instead of components
    world.insert_resource(CommandQueue {
        commands: VecDeque::new(),
    });
    world.insert_resource(CreatedPanes { panes: Vec::new() });

    // Create a schedule with our system
    let mut schedule = Schedule::default();
    schedule.add_systems(process_commands_system);

    println!("=== Command-Based Pane Creation Demo ===\n");

    // Enqueue commands instead of direct creation
    println!("Enqueueing commands...");
    enqueue_command(
        &mut world,
        Command::CreatePaneWithDatasets {
            dataset_ids: vec![
                DatasetId("temperature_sensor_1"),
                DatasetId("humidity_sensor_1"),
            ],
        },
    );

    enqueue_command(
        &mut world,
        Command::CreatePaneWithDatasets {
            dataset_ids: vec![DatasetId("humidity_sensor_1")],
        },
    );

    enqueue_command(
        &mut world,
        Command::CreatePaneWithDatasets {
            dataset_ids: vec![
                DatasetId("temperature_sensor_1"),
                DatasetId("pressure_sensor_1"),
            ],
        },
    );

    // Process commands through the system
    println!("\nExecuting command processing system...\n");
    schedule.run(&mut world);

    // Get created panes from the command system
    let created = world.resource::<CreatedPanes>().panes.clone();
    let pane_handles: Vec<PaneHandle> = created.iter().map(|(_, h)| *h).collect();

    let pane1 = pane_handles[0];
    let pane2 = pane_handles[1];
    let pane3 = pane_handles[2];

    // Print all panes
    println!("\n=== Panes ===");
    for (entity, pane) in world.query::<(Entity, &Pane)>().iter(&world) {
        let pane_handle = PaneHandle::new(entity);
        println!("Pane Handle: {:?}", pane_handle);
        println!("  Width: {}, Height: {}", pane.width, pane.height);

        // Query relationships: what datasets does this pane use?
        let mut used_datasets = Vec::new();
        if let Ok(entity_ref) = world.get_entity(entity) {
            if let Some(uses_dataset) = entity_ref.get::<UsesDataset>() {
                used_datasets.push(DatasetHandle::new(uses_dataset.dataset));
            }
        }

        if !used_datasets.is_empty() {
            println!(
                "  Uses {} datasets: {:?}",
                used_datasets.len(),
                used_datasets
            );
        } else {
            println!("  Uses no datasets");
        }
    }

    dump_subscriptions_by_dataset(&mut world);

    // Use command to delete pane 3
    println!("\n=== Demonstrating Command-Based Deletion ===");
    println!("Enqueueing delete command for pane 3...");
    enqueue_command(&mut world, Command::DeletePane { pane: pane3 });

    // Process the delete command
    println!("Executing command processing system...\n");
    schedule.run(&mut world);

    dump_subscriptions_by_dataset(&mut world);

    // Print world statistics
    println!("\n=== World Statistics ===");

    let pane_count = world.query::<&Pane>().iter(&world).count();
    println!("Entities with pane components: {}", pane_count);

    let dataset_count = world.query::<&DatasetId>().iter(&world).count();
    println!("Entities with dataset_id component: {}", dataset_count);

    // Count relationship instances
    let uses_relation_count = world.query::<&UsesDataset>().iter(&world).count();
    let subscriber_relation_count = world.query::<&DatasetSubscribers>().iter(&world).count();

    println!("UsesDataset relation instances: {}", uses_relation_count);
    println!(
        "DatasetSubscribers relation instances: {}",
        subscriber_relation_count
    );

    // List all entities and their components
    println!("\n=== All Entities ===");
    for entity in world.iter_entities() {
        print!("Entity {:?}: ", entity.id());

        let mut components = Vec::new();

        if entity.get::<Pane>().is_some() {
            components.push("Pane");
        }
        if entity.get::<DatasetId>().is_some() {
            components.push("DatasetId");
        }
        if entity.get::<UsesDataset>().is_some() {
            components.push("UsesDataset");
        }
        if entity.get::<DatasetSubscribers>().is_some() {
            components.push("DatasetSubscribers");
        }
        // CommandQueue and CreatedPanes are now Resources, not Components

        println!("Components: {:?}", components);
    }

    // Show archetype information
    println!("\n=== Archetype Analysis ===");
    let archetype_count = world.archetypes().len();
    println!("Total archetypes: {}", archetype_count);

    for (i, archetype) in world.archetypes().iter().enumerate() {
        println!("Archetype {}: {} entities", i, archetype.len());
    }

    // Demonstrate advanced queries
    println!("\n=== Query Examples ===");

    // Query all panes and their dimensions
    println!("All panes and their dimensions:");
    for (_, pane) in world.query::<(Entity, &Pane)>().iter(&world) {
        println!("  Pane: {}x{}", pane.width, pane.height);
    }

    // Query all datasets and show their IDs
    println!("All datasets:");
    for (_, dataset_id) in world.query::<(Entity, &DatasetId)>().iter(&world) {
        println!("  Dataset: {:#?}", dataset_id);
    }

    // Demonstrate type safety - these would be compile errors:
    // let wrong_panes = get_panes_for_dataset(&world, pane1); // Error: expected DatasetHandle, found PaneHandle
    // let mixed_handles: Vec<Entity> = vec![pane1, dataset1]; // Error: can't mix handle types

    println!("\n=== Bevy ECS Example Complete ===");
    println!("This demonstrates enhanced Bevy ECS functionality:");
    println!(
        "- TYPE-SAFE ENTITY HANDLES: PaneHandle and DatasetHandle prevent mixing entity types"
    );
    println!(
        "- BEVY SYSTEMS: Proper system functions with Commands, Res, ResMut, Query parameters"
    );
    println!("- SCHEDULE INTEGRATION: System execution via Schedule.run() like real Bevy apps");
    println!(
        "- BUILT-IN RELATIONSHIPS: #[relationship] and #[relationship_target] for semantic connections"
    );
    println!("- Component definition using #[derive(Component)]");
    println!("- Resources for global state with #[derive(Resource)]");
    println!("- Entity creation with Commands.spawn() method");
    println!("- Query system with flexible component combinations");
    println!("- World introspection and archetype analysis");
    println!("- Automatic bidirectional relationship management");
    println!("- Modern Rust API with comprehensive derive macros");
}
