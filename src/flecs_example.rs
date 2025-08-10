#![allow(unused)]
use flecs::*;
use std::collections::{HashMap, VecDeque};

// Macro to create type-safe entity handles
macro_rules! entity_handles {
    ($($handle_name:ident),* $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DatasetId(&'static str);

// Flecs Rust bindings are extremely limited - use basic types that work
// Component trait is automatically implemented for 'static types
#[derive(Debug, Clone)]
struct Pane {
    width: u32,
    height: u32,
}

#[derive(Debug, Clone)]
struct PaneDatasets {
    dataset_handles: Vec<DatasetHandle>,
}

// Command system components - limited by Flecs API
#[derive(Debug, Clone)]
struct CommandQueue {
    commands: VecDeque<Command>,
}

#[derive(Debug, Clone)]
struct CreatedPanes {
    panes: Vec<(Vec<DatasetId>, PaneHandle)>,
}

// Command types
#[derive(Debug, Clone)]
pub enum Command {
    CreatePaneWithDatasets { dataset_ids: Vec<DatasetId> },
    DeletePane { pane: PaneHandle },
}

// Create a very simple implementation due to extremely limited Flecs Rust API
fn create_pane_with_datasets(
    world: &World,
    dataset_ids: Vec<DatasetId>,
    created_datasets: &mut HashMap<DatasetId, DatasetHandle>,
) -> (PaneHandle, Vec<DatasetHandle>) {
    // Create the pane entity
    let pane = world.entity().set(Pane {
        width: 100,
        height: 200,
    });
    let pane_handle = PaneHandle::new(pane);

    // Create dataset entities (limited deduplication due to API limitations)
    let mut dataset_handles = Vec::new();

    for dataset_id in dataset_ids {
        let dataset_handle = if let Some(&existing_handle) = created_datasets.get(&dataset_id) {
            existing_handle
        } else {
            let dataset = world.entity().set(dataset_id);
            let dataset_handle = DatasetHandle::new(dataset);
            created_datasets.insert(dataset_id, dataset_handle);
            dataset_handle
        };

        dataset_handles.push(dataset_handle);
    }

    // Store the relationships in the pane
    pane.set(PaneDatasets {
        dataset_handles: dataset_handles.clone(),
    });

    (pane_handle, dataset_handles)
}

fn get_panes_for_dataset(
    world: &World,
    dataset: DatasetHandle,
    all_panes: &[(PaneHandle, Vec<DatasetHandle>)],
) -> Vec<PaneHandle> {
    let mut subscribing_panes = Vec::new();

    for &(pane_handle, ref dataset_handles) in all_panes {
        if dataset_handles.contains(&dataset) {
            subscribing_panes.push(pane_handle);
        }
    }

    subscribing_panes
}

// Command processing system (simplified due to API limitations)
fn process_commands_system(
    world: &World,
    commands: &mut VecDeque<Command>,
    created_datasets: &mut HashMap<DatasetId, DatasetHandle>,
    created_panes: &mut Vec<(Vec<DatasetId>, PaneHandle)>,
    all_pane_dataset_relations: &mut Vec<(PaneHandle, Vec<DatasetHandle>)>,
) {
    // Process commands and collect results
    let mut new_panes = Vec::new();
    let mut deleted_panes = Vec::new();

    for cmd in commands.drain(..) {
        match cmd {
            Command::CreatePaneWithDatasets { dataset_ids } => {
                println!(
                    "[System] Processing CreatePaneWithDatasets command with {} datasets",
                    dataset_ids.len()
                );
                let (pane_handle, dataset_handles) =
                    create_pane_with_datasets(world, dataset_ids.clone(), created_datasets);
                new_panes.push((dataset_ids.clone(), pane_handle));
                all_pane_dataset_relations.push((pane_handle, dataset_handles));
                println!("[System] Created pane: {:?}", pane_handle);
            }
            Command::DeletePane { pane } => {
                println!("[System] Processing DeletePane command for {:?}", pane);
                // Note: Due to API limitations, we can't actually despawn entities
                // In a real implementation with full Flecs API, you would call world.delete(pane.entity())
                deleted_panes.push(pane);
                println!(
                    "[System] Note: Entity despawn not supported in current Flecs Rust bindings"
                );
            }
        }
    }

    // Update tracking after processing
    for new_pane in new_panes {
        created_panes.push(new_pane);
    }
    for deleted_pane in deleted_panes {
        created_panes.retain(|(_, h)| *h != deleted_pane);
        all_pane_dataset_relations.retain(|(h, _)| *h != deleted_pane);
    }
}

// Helper to enqueue commands
fn enqueue_command(commands: &mut VecDeque<Command>, cmd: Command) {
    commands.push_back(cmd);
}

fn dump_subscriptions_by_dataset(
    created_datasets: &HashMap<DatasetId, DatasetHandle>,
    all_pane_dataset_relations: &[(PaneHandle, Vec<DatasetHandle>)],
) {
    // Print all datasets and their subscriptions
    println!("\n=== Dataset Subscriptions ===");

    for (&dataset_id, &dataset_handle) in created_datasets {
        println!("Dataset: {:#?}", dataset_id);
        println!("  Handle: {:?}", dataset_handle);

        // Use the dedicated function to get panes for this dataset
        let subscribing_panes =
            get_panes_for_dataset(&World::new(), dataset_handle, all_pane_dataset_relations);

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
    // Create a new flecs world
    let mut world = World::new();

    // Register components - required by Flecs Rust bindings
    world.component::<Pane>();
    world.component::<DatasetId>();
    world.component::<PaneDatasets>();

    // Command system state (manual management due to API limitations)
    let mut command_queue = VecDeque::new();
    let mut created_panes = Vec::new();
    let mut created_datasets = HashMap::new();
    let mut all_pane_dataset_relations = Vec::new();

    // Create some panes with datasets - simplified due to API limitations
    println!("=== Command-Based Pane Creation Demo ===\n");
    println!(
        "Note: Flecs Rust bindings are extremely limited - this is an enhanced demonstration within constraints"
    );

    // Enqueue commands instead of direct creation
    println!("Enqueueing commands...");
    enqueue_command(
        &mut command_queue,
        Command::CreatePaneWithDatasets {
            dataset_ids: vec![
                DatasetId("temperature_sensor_1"),
                DatasetId("humidity_sensor_1"),
            ],
        },
    );

    enqueue_command(
        &mut command_queue,
        Command::CreatePaneWithDatasets {
            dataset_ids: vec![DatasetId("humidity_sensor_1")],
        },
    );

    enqueue_command(
        &mut command_queue,
        Command::CreatePaneWithDatasets {
            dataset_ids: vec![
                DatasetId("temperature_sensor_1"),
                DatasetId("pressure_sensor_1"),
            ],
        },
    );

    // Process commands through the system
    println!("\nExecuting command processing system...\n");
    process_commands_system(
        &world,
        &mut command_queue,
        &mut created_datasets,
        &mut created_panes,
        &mut all_pane_dataset_relations,
    );

    // Get created panes from the command system
    let pane_handles: Vec<PaneHandle> = created_panes.iter().map(|(_, h)| *h).collect();

    let pane1 = pane_handles[0];
    let pane2 = pane_handles[1];
    let pane3 = pane_handles[2];

    // Print all panes
    println!("\n=== Panes ===");
    for &(ref dataset_ids, pane_handle) in &created_panes {
        let entity = pane_handle.entity();
        let pane = entity.get::<Pane>();
        println!("Pane Handle: {:?}", pane_handle);
        println!("  Width: {}, Height: {}", pane.width, pane.height);
        println!("  Uses {} datasets: {:?}", dataset_ids.len(), dataset_ids);
    }

    dump_subscriptions_by_dataset(&created_datasets, &all_pane_dataset_relations);

    // Use command to delete pane 3
    println!("\n=== Demonstrating Command-Based Deletion ===");
    println!("Enqueueing delete command for pane 3...");
    enqueue_command(&mut command_queue, Command::DeletePane { pane: pane3 });

    // Process the delete command
    println!("Executing command processing system...\n");
    process_commands_system(
        &world,
        &mut command_queue,
        &mut created_datasets,
        &mut created_panes,
        &mut all_pane_dataset_relations,
    );

    dump_subscriptions_by_dataset(&created_datasets, &all_pane_dataset_relations);

    // Print world statistics
    println!("\n=== World Statistics ===");
    println!("Note: Flecs Rust bindings are extremely limited");

    println!("Entities with Pane component: {}", created_panes.len());
    println!(
        "Entities with DatasetId component: {}",
        created_datasets.len()
    );
    println!(
        "Total tracked entities: {}",
        created_panes.len() + created_datasets.len()
    );

    // List all entities and their components
    println!("\n=== All Tracked Entities ===");

    // List pane entities
    for &(ref dataset_ids, pane_handle) in &created_panes {
        println!(
            "Entity {:?}: Components: [\"Pane\", \"PaneDatasets\"]",
            pane_handle.entity()
        );
    }

    // List dataset entities
    for (dataset_id, dataset_handle) in &created_datasets {
        println!(
            "Entity {:?}: Components: [\"DatasetId\"] (ID: {:?})",
            dataset_handle.entity(),
            dataset_id
        );
    }

    // Demonstrate basic queries - simplified
    println!("\n=== Query Examples ===");

    // Show all panes and their dimensions
    println!("All panes and their dimensions:");
    for &(_, pane_handle) in &created_panes {
        let entity = pane_handle.entity();
        let pane = entity.get::<Pane>();
        println!("  Pane: {}x{}", pane.width, pane.height);
    }

    // Show all datasets and their IDs
    println!("All datasets:");
    for (&dataset_id, _) in &created_datasets {
        println!("  Dataset: {:#?}", dataset_id);
    }

    // Demonstrate type safety - these would be compile errors:
    // let wrong_panes = get_panes_for_dataset(&world, pane1, &all_pane_dataset_relations); // Error: expected DatasetHandle, found PaneHandle
    // let mixed_handles: Vec<Entity> = vec![pane1.entity(), dataset1.entity()]; // Error: can't mix handle types

    println!("\n=== Flecs Example Complete ===");
    println!("This demonstrates enhanced Flecs ECS functionality (within API constraints):");
    println!(
        "- TYPE-SAFE ENTITY HANDLES: PaneHandle and DatasetHandle prevent mixing entity types"
    );
    println!("- COMMAND SYSTEM: Queue-based command processing with systems");
    println!("- Component definition (Component trait auto-implemented)");
    println!("- Entity creation with .entity().set() pattern");
    println!("- Basic component access with .get()");
    println!("- Manual relationship management with Vec<Handle> (due to API limitations)");
    println!("");
    println!("IMPORTANT LIMITATIONS:");
    println!("- No #[derive(Component)] macro available");
    println!("- No .has() method for checking components");
    println!("- No query API (no .query(), .each(), .filter())");
    println!("- No relationship API");
    println!("- No entity despawn in current bindings");
    println!("- Current Flecs Rust bindings (0.1.x) are incomplete and not production-ready");
    println!("- For production use, consider the C API directly or wait for better Rust bindings");
}
