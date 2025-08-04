#![allow(unused)]
use evenio::prelude::*;
use std::collections::VecDeque;

// Macro to create type-safe entity handles
macro_rules! entity_handles {
    ($($handle_name:ident),* $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub struct $handle_name(EntityId);

            impl $handle_name {
                pub fn new(entity: EntityId) -> Self {
                    Self(entity)
                }

                pub fn entity(&self) -> EntityId {
                    self.0
                }
            }

            impl From<EntityId> for $handle_name {
                fn from(entity: EntityId) -> Self {
                    Self(entity)
                }
            }

            impl From<$handle_name> for EntityId {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
#[component(immutable)]
struct DatasetId(&'static str);

#[derive(Component, Default)]
struct AllPanes {
    panes: Vec<PaneHandle>,
}

#[derive(Component)]
struct Pane {
    width: u32,
    height: u32,
}

#[derive(Component)]
struct PaneDatasets {
    datasets: Vec<DatasetHandle>,
}

#[derive(Component)]
struct DatasetSubscription {
    panes: Vec<PaneHandle>,
}

#[derive(Component, Default)]
struct DatasetIdToDatasetEntityLookup {
    lookup: std::collections::HashMap<DatasetId, DatasetHandle>,
}

// Command system components
#[derive(Component)]
struct CommandQueue {
    commands: VecDeque<Command>,
}

#[derive(Component)]
struct CreatedPanes {
    panes: Vec<(Vec<DatasetId>, PaneHandle)>,
}

// Command types
#[derive(Debug, Clone)]
pub enum Command {
    CreatePaneWithDatasets { dataset_ids: Vec<DatasetId> },
    DeletePane { pane: PaneHandle },
}

// Events can carry data, but for this example we only need a unit struct.
#[derive(GlobalEvent)]
struct CreatePaneWithDataset {
    datasets: Vec<DatasetId>,
}

#[derive(GlobalEvent)]
struct ProcessCommands;

struct AppRegistry {
    pane_lookup: EntityId,
    dataset_lookup: EntityId,
    command_queue: EntityId,
    world: World,
}

fn create_pane_with_datasets(
    world: &mut World,
    dataset_ids: Vec<DatasetId>,
    pane_lookup: EntityId,
    dataset_lookup: EntityId,
) -> PaneHandle {
    // Create the pane entity
    let pane_entity = world.spawn();
    world.insert(pane_entity, Pane { width: 100, height: 200 });
    let pane_handle = PaneHandle::new(pane_entity);

    let mut dataset_handles = Vec::new();

    for dataset_id in dataset_ids {
        // Check if dataset already exists
        let existing_dataset = {
            let lookup = world.get::<DatasetIdToDatasetEntityLookup>(dataset_lookup).unwrap();
            lookup.lookup.get(&dataset_id).cloned()
        };

        let dataset_handle = if let Some(existing) = existing_dataset {
            existing
        } else {
            // Create a new dataset entity
            let dataset_entity = world.spawn();
            world.insert(dataset_entity, dataset_id.clone());
            let dataset_handle = DatasetHandle::new(dataset_entity);
            
            // Update lookup
            let mut lookup = world.get_mut::<DatasetIdToDatasetEntityLookup>(dataset_lookup).unwrap();
            lookup.lookup.insert(dataset_id, dataset_handle);
            dataset_handle
        };

        dataset_handles.push(dataset_handle);
    }

    world.insert(pane_entity, PaneDatasets { datasets: dataset_handles });

    // Add pane to the all_panes registry
    let mut all_panes = world.get_mut::<AllPanes>(pane_lookup).unwrap();
    all_panes.panes.push(pane_handle);

    pane_handle
}

fn get_panes_for_dataset(world: &World, dataset: DatasetHandle, pane_lookup: EntityId) -> Vec<PaneHandle> {
    let mut subscribing_panes = Vec::new();
    
    let all_panes = world.get::<AllPanes>(pane_lookup).unwrap();
    for &pane_handle in &all_panes.panes {
        if let Some(pane_datasets) = world.get::<PaneDatasets>(pane_handle.entity()) {
            if pane_datasets.datasets.contains(&dataset) {
                subscribing_panes.push(pane_handle);
            }
        }
    }
    
    subscribing_panes
}

// Command processing system
fn process_commands_system(
    world: &mut World,
    command_entity: EntityId,
    pane_lookup: EntityId,
    dataset_lookup: EntityId,
) {
    // Get and process all pending commands
    let commands: Vec<Command> = {
        let mut queue = world.get_mut::<CommandQueue>(command_entity).unwrap();
        queue.commands.drain(..).collect()
    };
    
    // Process commands and collect results
    let mut new_panes = Vec::new();
    let mut deleted_panes = Vec::new();
    
    for cmd in commands {
        match cmd {
            Command::CreatePaneWithDatasets { dataset_ids } => {
                println!("[System] Processing CreatePaneWithDatasets command with {} datasets", dataset_ids.len());
                let pane_handle = create_pane_with_datasets(world, dataset_ids.clone(), pane_lookup, dataset_lookup);
                new_panes.push((dataset_ids, pane_handle));
                println!("[System] Created pane: {:?}", pane_handle);
            }
            Command::DeletePane { pane } => {
                println!("[System] Processing DeletePane command for {:?}", pane);
                world.despawn(pane.entity());
                deleted_panes.push(pane);
            }
        }
    }
    
    // Update created_panes tracking after processing
    {
        let mut created = world.get_mut::<CreatedPanes>(command_entity).unwrap();
        for new_pane in new_panes {
            created.panes.push(new_pane);
        }
        for deleted_pane in &deleted_panes {
            created.panes.retain(|(_, h)| *h != *deleted_pane);
        }
    }
    
    // Remove deleted panes from all_panes registry
    for deleted_pane in deleted_panes {
        let mut all_panes = world.get_mut::<AllPanes>(pane_lookup).unwrap();
        all_panes.panes.retain(|&h| h != deleted_pane);
    }
}

// Helper to enqueue commands
fn enqueue_command(world: &mut World, command_entity: EntityId, cmd: Command) {
    let mut queue = world.get_mut::<CommandQueue>(command_entity).unwrap();
    queue.commands.push_back(cmd);
}

fn dump_subscriptions_by_dataset(world: &World, dataset_lookup: EntityId, pane_lookup: EntityId) {
    // Print all datasets and their subscriptions
    println!("\n=== Dataset Subscriptions ===");

    let lookup = world.get::<DatasetIdToDatasetEntityLookup>(dataset_lookup).unwrap();
    for (&dataset_id, &dataset_handle) in &lookup.lookup {
        println!("Dataset: {:#?}", dataset_id);
        println!("  Handle: {:?}", dataset_handle);

        // Use the dedicated function to get panes for this dataset
        let subscribing_panes = get_panes_for_dataset(&world, dataset_handle, pane_lookup);

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
    // Create a new `World` to store all our data.
    let mut world = World::new();

    let dataset_lookup = world.spawn();
    world.insert(dataset_lookup, DatasetIdToDatasetEntityLookup::default());
    let pane_lookup = world.spawn();
    world.insert(pane_lookup, AllPanes::default());

    // Create command queue entity
    let command_entity = world.spawn();
    world.insert(command_entity, CommandQueue { commands: VecDeque::new() });
    world.insert(command_entity, CreatedPanes { panes: Vec::new() });

    let mut registry = AppRegistry {
        pane_lookup,
        dataset_lookup,
        command_queue: command_entity,
        world,
    };

    println!("=== Command-Based Pane Creation Demo ===\n");
    
    // Enqueue commands instead of direct creation
    println!("Enqueueing commands...");
    enqueue_command(&mut registry.world, command_entity, Command::CreatePaneWithDatasets {
        dataset_ids: vec![
            DatasetId("temperature_sensor_1"),
            DatasetId("humidity_sensor_1"),
        ],
    });
    
    enqueue_command(&mut registry.world, command_entity, Command::CreatePaneWithDatasets {
        dataset_ids: vec![DatasetId("humidity_sensor_1")],
    });
    
    enqueue_command(&mut registry.world, command_entity, Command::CreatePaneWithDatasets {
        dataset_ids: vec![
            DatasetId("temperature_sensor_1"),
            DatasetId("pressure_sensor_1"),
        ],
    });
    
    // Process commands through the system
    println!("\nExecuting command processing system...\n");
    process_commands_system(&mut registry.world, command_entity, pane_lookup, dataset_lookup);
    
    // Get created panes from the command system
    let created = registry.world.get::<CreatedPanes>(command_entity).unwrap().panes.clone();
    let pane_handles: Vec<PaneHandle> = created.iter().map(|(_, h)| *h).collect();
    
    let pane1 = pane_handles[0];
    let pane2 = pane_handles[1];
    let pane3 = pane_handles[2];

    // Dump all panes and dataset subscriptions
    println!("\n=== Panes ===");
    for &pane_handle in &registry
        .world
        .get::<AllPanes>(registry.pane_lookup)
        .unwrap()
        .panes
    {
        let pane = registry.world.get::<Pane>(pane_handle.entity()).unwrap();
        let pane_datasets = registry.world.get::<PaneDatasets>(pane_handle.entity()).unwrap();
        println!("Pane Handle: {:?}", pane_handle);
        println!("  Width: {}, Height: {}", pane.width, pane.height);
        println!("  Uses {} datasets: {:?}", pane_datasets.datasets.len(), pane_datasets.datasets);
    }

    dump_subscriptions_by_dataset(&registry.world, dataset_lookup, pane_lookup);

    // Use command to delete pane 3
    println!("\n=== Demonstrating Command-Based Deletion ===");
    println!("Enqueueing delete command for pane 3...");
    enqueue_command(&mut registry.world, command_entity, Command::DeletePane { pane: pane3 });
    
    // Process the delete command
    println!("Executing command processing system...\n");
    process_commands_system(&mut registry.world, command_entity, pane_lookup, dataset_lookup);

    dump_subscriptions_by_dataset(&registry.world, dataset_lookup, pane_lookup);

    // Print world statistics
    println!("\n=== World Statistics ===");
    
    let all_panes = registry.world.get::<AllPanes>(registry.pane_lookup).unwrap();
    println!("Entities with pane components: {}", all_panes.panes.len());

    let lookup = registry.world.get::<DatasetIdToDatasetEntityLookup>(registry.dataset_lookup).unwrap();
    println!("Entities with dataset_id component: {}", lookup.lookup.len());

    println!("Total entities: {}", registry.world.entities().len());

    println!("\n=== All Entity Locations ===");
    for location in registry.world.entities().iter() {
        println!("Entity Location: {:?}", location);
    }

    println!("\n=== All Archetypes ===");
    let archetypes = registry.world.archetypes();
    println!("Total archetypes: {}", archetypes.len());
    for (i, archetype) in archetypes.iter().enumerate() {
        println!("Archetype {} (Index {:?}):", i, archetype.index());
        println!("  Component count: {}", archetype.component_indices().len());
        println!("  Entity count: {}", archetype.entity_count());
        println!("  Component indices: {:?}", archetype.component_indices());
        if archetype.entity_count() > 0 {
            println!(
                "  First few entity IDs: {:?}",
                &archetype.entity_ids()[..archetype.entity_count().min(5) as usize]
            );
        }
    }

    // Demonstrate advanced queries
    println!("\n=== Query Examples ===");
    
    // Query all panes and their dimensions
    println!("All panes and their dimensions:");
    for &pane_handle in &all_panes.panes {
        let pane = registry.world.get::<Pane>(pane_handle.entity()).unwrap();
        println!("  Pane: {}x{}", pane.width, pane.height);
    }
    
    // Query all datasets and show their IDs
    println!("All datasets:");
    for (&dataset_id, _) in &lookup.lookup {
        println!("  Dataset: {:#?}", dataset_id);
    }

    // Demonstrate type safety - these would be compile errors:
    // let wrong_panes = get_panes_for_dataset(&registry.world, pane1, pane_lookup); // Error: expected DatasetHandle, found PaneHandle
    // let mixed_handles: Vec<EntityId> = vec![pane1.entity(), dataset1.entity()]; // Error: can't mix handle types
    
    println!("\n=== Evenio Example Complete ===");
    println!("This demonstrates enhanced Evenio ECS functionality:");
    println!("- TYPE-SAFE ENTITY HANDLES: PaneHandle and DatasetHandle prevent mixing entity types");
    println!("- COMMAND SYSTEM: Queue-based command processing with systems");
    println!("- Component definition with derive macros");
    println!("- Entity creation with .spawn() method");
    println!("- Event-driven architecture with handlers");
    println!("- Registry pattern for entity management");
    println!("- World introspection and archetype analysis");
    println!("- Manual relationship management with Vec<Handle>");
}