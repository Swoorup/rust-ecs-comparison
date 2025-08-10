#![allow(unused)]
use hecs::*;
use hecs_hierarchy::*;
use std::collections::VecDeque;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DatasetId(&'static str);

#[derive(Debug, Clone)]
struct Pane {
    width: u32,
    height: u32,
}

// Marker components for hierarchy organization
#[derive(Debug, Clone)]
struct PaneRoot;

#[derive(Debug, Clone)]
struct DatasetRoot;

// Command system components
#[derive(Debug, Clone)]
struct CommandQueue {
    commands: VecDeque<Command>,
}

#[derive(Debug, Clone)]
struct CreatedPanes {
    panes: Vec<(Vec<DatasetId>, PaneHandle)>,
}

// Hierarchy marker type - allows multiple hierarchies to coexist
struct Tree;

// Command types
#[derive(Debug, Clone)]
pub enum Command {
    CreatePaneWithDatasets { dataset_ids: Vec<DatasetId> },
    DeletePane { pane: PaneHandle },
}

fn create_pane_with_datasets(
    world: &mut World,
    dataset_ids: Vec<DatasetId>,
    pane_root: Entity,
    dataset_root: Entity,
) -> PaneHandle {
    // Create the pane entity and attach it as child of pane_root
    let pane = world
        .attach_new::<Tree, _>(
            pane_root,
            (Pane {
                width: 100,
                height: 200,
            },),
        )
        .unwrap();
    let pane_handle = PaneHandle::new(pane);

    for dataset_id in dataset_ids {
        // Find existing dataset by searching children of dataset_root
        let mut existing_dataset = None;

        // Iterate through children of dataset_root to find matching dataset
        for child in world.children::<Tree>(dataset_root) {
            if let Ok(existing_id) = world.get::<&DatasetId>(child) {
                if *existing_id == dataset_id {
                    existing_dataset = Some(DatasetHandle::new(child));
                    break;
                }
            }
        }

        let dataset_handle = if let Some(existing) = existing_dataset {
            existing
        } else {
            // Create new dataset entity as child of dataset_root
            let dataset_entity = world
                .attach_new::<Tree, _>(dataset_root, (dataset_id,))
                .unwrap();
            DatasetHandle::new(dataset_entity)
        };

        // Create relationship: attach pane as child of dataset to show "uses" relationship
        // This creates a many-to-many relationship through the hierarchy
        world.attach::<Tree>(pane, dataset_handle.entity()).unwrap();
    }

    pane_handle
}

fn get_panes_for_dataset(world: &World, dataset: DatasetHandle) -> Vec<PaneHandle> {
    let mut subscribing_panes = Vec::new();
    // Get all children of this dataset (which are panes that use it)
    for child in world.children::<Tree>(dataset.entity()) {
        if world.get::<&Pane>(child).is_ok() {
            subscribing_panes.push(PaneHandle::new(child));
        }
    }
    subscribing_panes
}

// Command processing system
fn process_commands_system(
    world: &mut World,
    command_entity: Entity,
    pane_root: Entity,
    dataset_root: Entity,
) {
    // Get and process all pending commands
    let commands: Vec<Command> = {
        let mut queue = world.get::<&mut CommandQueue>(command_entity).unwrap();
        queue.commands.drain(..).collect()
    };

    // Process commands and collect results
    let mut new_panes = Vec::new();
    let mut deleted_panes = Vec::new();

    for cmd in commands {
        match cmd {
            Command::CreatePaneWithDatasets { dataset_ids } => {
                println!(
                    "[System] Processing CreatePaneWithDatasets command with {} datasets",
                    dataset_ids.len()
                );
                let pane_handle =
                    create_pane_with_datasets(world, dataset_ids.clone(), pane_root, dataset_root);
                new_panes.push((dataset_ids, pane_handle));
                println!("[System] Created pane: {:?}", pane_handle);
            }
            Command::DeletePane { pane } => {
                println!("[System] Processing DeletePane command for {:?}", pane);
                world.despawn(pane.entity()).ok();
                deleted_panes.push(pane);
            }
        }
    }

    // Update created_panes tracking after processing
    let mut created = world.get::<&mut CreatedPanes>(command_entity).unwrap();
    for new_pane in new_panes {
        created.panes.push(new_pane);
    }
    for deleted_pane in deleted_panes {
        created.panes.retain(|(_, h)| *h != deleted_pane);
    }
}

// Helper to enqueue commands
fn enqueue_command(world: &mut World, command_entity: Entity, cmd: Command) {
    let mut queue = world.get::<&mut CommandQueue>(command_entity).unwrap();
    queue.commands.push_back(cmd);
}

fn dump_subscriptions_by_dataset(world: &World, dataset_root: Entity) {
    // Print all datasets and their subscriptions
    println!("\n=== Dataset Subscriptions ===");

    for dataset_entity in world.children::<Tree>(dataset_root) {
        if let Ok(dataset_id) = world.get::<&DatasetId>(dataset_entity) {
            println!("Dataset: {:#?}", dataset_id);
            println!("  Handle: {:?}", DatasetHandle::new(dataset_entity));

            // Use the dedicated function to get panes for this dataset
            let subscribing_panes =
                get_panes_for_dataset(&world, DatasetHandle::new(dataset_entity));

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
}

pub fn main() {
    // Create a new hecs world
    let mut world = World::new();

    // Create root entities for organization
    let pane_root = world.spawn((PaneRoot,));
    let dataset_root = world.spawn((DatasetRoot,));

    // Create command queue entity
    let command_entity = world.spawn((
        CommandQueue {
            commands: VecDeque::new(),
        },
        CreatedPanes { panes: Vec::new() },
    ));

    println!("=== Command-Based Pane Creation Demo ===\n");

    // Enqueue commands instead of direct creation
    println!("Enqueueing commands...");
    enqueue_command(
        &mut world,
        command_entity,
        Command::CreatePaneWithDatasets {
            dataset_ids: vec![
                DatasetId("temperature_sensor_1"),
                DatasetId("humidity_sensor_1"),
            ],
        },
    );

    enqueue_command(
        &mut world,
        command_entity,
        Command::CreatePaneWithDatasets {
            dataset_ids: vec![DatasetId("humidity_sensor_1")],
        },
    );

    enqueue_command(
        &mut world,
        command_entity,
        Command::CreatePaneWithDatasets {
            dataset_ids: vec![
                DatasetId("temperature_sensor_1"),
                DatasetId("pressure_sensor_1"),
            ],
        },
    );

    // Process commands through the system
    println!("\nExecuting command processing system...\n");
    process_commands_system(&mut world, command_entity, pane_root, dataset_root);

    // Get created panes from the command system
    let created = world
        .get::<&CreatedPanes>(command_entity)
        .unwrap()
        .panes
        .clone();
    let pane_handles: Vec<PaneHandle> = created.iter().map(|(_, h)| *h).collect();

    let pane1 = pane_handles[0];
    let pane2 = pane_handles[1];
    let pane3 = pane_handles[2];

    // Print all panes using hierarchy
    println!("\n=== Panes (via Hierarchy) ===");
    for pane_entity in world.children::<Tree>(pane_root) {
        if let Ok(pane) = world.get::<&Pane>(pane_entity) {
            let pane_handle = PaneHandle::new(pane_entity);
            println!("Pane Handle: {:?}", pane_handle);
            println!("  Width: {}, Height: {}", pane.width, pane.height);

            // Find datasets this pane uses by looking at which datasets have this pane as child
            let mut used_datasets = Vec::new();
            for dataset_entity in world.children::<Tree>(dataset_root) {
                // Check if this pane is a child of this dataset
                let dataset_children: Vec<_> = world.children::<Tree>(dataset_entity).collect();
                if dataset_children.contains(&pane_entity) {
                    used_datasets.push(DatasetHandle::new(dataset_entity));
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
    }

    dump_subscriptions_by_dataset(&world, dataset_root);

    // Use command to delete pane 3
    println!("\n=== Demonstrating Command-Based Deletion ===");
    println!("Enqueueing delete command for pane 3...");
    enqueue_command(
        &mut world,
        command_entity,
        Command::DeletePane { pane: pane3 },
    );

    // Process the delete command
    println!("Executing command processing system...\n");
    process_commands_system(&mut world, command_entity, pane_root, dataset_root);

    dump_subscriptions_by_dataset(&world, dataset_root);

    // Print world statistics
    println!("\n=== World Statistics ===");

    // Count entities
    let entity_count = world.len();
    println!("Total entities: {}", entity_count);

    // Count entities with Pane component
    let mut pane_count = 0;
    for (_, _) in world.query::<&Pane>().iter() {
        pane_count += 1;
    }
    println!("Entities with Pane component: {}", pane_count);

    // Count entities with DatasetId component
    let mut dataset_count = 0;
    for (_, _) in world.query::<&DatasetId>().iter() {
        dataset_count += 1;
    }
    println!("Entities with DatasetId component: {}", dataset_count);

    // List all entities and their components
    println!("\n=== All Entities ===");
    for entity in world.iter() {
        let entity_id = entity.entity();
        print!("Entity {:?}: ", entity_id);

        // Check for each component type
        let mut components = Vec::new();

        if entity.get::<&Pane>().is_some() {
            components.push("Pane");
        }

        if entity.get::<&DatasetId>().is_some() {
            components.push("DatasetId");
        }

        if entity.get::<&PaneRoot>().is_some() {
            components.push("PaneRoot");
        }

        if entity.get::<&DatasetRoot>().is_some() {
            components.push("DatasetRoot");
        }

        if entity.get::<&CommandQueue>().is_some() {
            components.push("CommandQueue");
        }

        if entity.get::<&CreatedPanes>().is_some() {
            components.push("CreatedPanes");
        }

        // Show hierarchy information
        if let Ok(parent) = world.parent::<Tree>(entity_id) {
            components.push("HasParent");
        }

        let children: Vec<_> = world.children::<Tree>(entity_id).collect();
        if !children.is_empty() {
            components.push("HasChildren");
        }

        println!("Components: {:?}", components);
    }

    // Demonstrate type safety - these would be compile errors:
    // let wrong_panes = get_panes_for_dataset(&world, pane1); // Error: expected DatasetHandle, found PaneHandle
    // let mixed_handles: Vec<Entity> = vec![pane1, dataset1]; // Error: can't mix handle types

    println!("\n=== Hecs Hierarchy Example Complete ===");
    println!("This demonstrates enhanced Hecs ECS with hecs-hierarchy functionality:");
    println!(
        "- TYPE-SAFE ENTITY HANDLES: PaneHandle and DatasetHandle prevent mixing entity types"
    );
    println!("- COMMAND SYSTEM: Queue-based command processing with systems");
    println!("- Component definition with plain structs");
    println!("- Entity creation with .spawn() and .attach_new() methods");
    println!("- Hierarchy management with Tree marker type");
    println!("- Parent-child relationships via .attach() method");
    println!("- Query system for components and hierarchy traversal");
    println!("- Many-to-many relationships via hierarchy (pane can use multiple datasets)");
    println!("- Efficient relationship queries through .children() and .parent()");
    println!("- No manual Vec<Entity> bookkeeping required");
    println!("- Built-in depth-first and breadth-first traversal");
}
