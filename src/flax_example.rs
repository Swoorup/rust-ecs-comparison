#![allow(unused)]
use flax::*;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DatasetId(&'static str);

pub mod pane {
    use flax::component;

    component! {
        // Pane components
        pub width: u32,
        pub height: u32,
        pub(crate) uses_dataset(dataset): (),
    }
}

pub mod dataset {
    use crate::DatasetId;
    use flax::component;

    component! {
        // Dataset components
        pub id: DatasetId,
        // Relations
        pub(crate) subscribed_by(pane): (),
    }
}

// Command system components
component! {
    // Command queue - singleton entity holds all commands
    pane_command_queue: VecDeque<Command>,
    // Static entity, which is always alive
    resources,
}

// Command types
#[derive(Debug, Clone)]
pub enum Command {
    CreatePaneWithDatasets { dataset_ids: Vec<DatasetId> },
    DeletePane { pane: PaneHandle },
}

fn create_pane_with_datasets(
    world: &mut World,
    dataset_ids: Vec<DatasetId>,
    width: u32,
    height: u32,
) -> PaneHandle {
    // Create the pane entity
    let pane_entity = Entity::builder()
        .set(pane::width(), width)
        .set(pane::height(), height)
        .spawn(world);
    let pane = PaneHandle::new(pane_entity);

    for ds in dataset_ids {
        // Find existing dataset by querying all datasets
        let mut existing_dataset = None;
        {
            let mut query = Query::new((entity_ids(), dataset::id()));
            let mut binding = query.borrow(world);
            for (entity, &id) in binding.iter() {
                if id == ds {
                    existing_dataset = Some(DatasetHandle::new(entity));
                    break;
                }
            }
        }

        let dataset = if let Some(existing) = existing_dataset {
            existing
        } else {
            // Create new dataset entity
            let dataset_entity = Entity::builder().set(dataset::id(), ds).spawn(world);
            DatasetHandle::new(dataset_entity)
        };

        // Create the relation: pane uses dataset
        world
            .set(pane.entity(), pane::uses_dataset(dataset.entity()), ())
            .unwrap();

        // Create the reverse relation: dataset is subscribed by pane
        world
            .set(dataset.entity(), dataset::subscribed_by(pane.entity()), ())
            .unwrap();
    }

    pane
}

fn get_panes_for_dataset(world: &World, dataset: DatasetHandle) -> Vec<PaneHandle> {
    let mut subscribing_panes = Vec::new();
    let mut relation_query = Query::new(relations_like(dataset::subscribed_by));
    if let Ok(relations) = relation_query.borrow(world).get(dataset.entity()) {
        for (target, _) in relations {
            subscribing_panes.push(PaneHandle::new(target));
        }
    }
    subscribing_panes
}

// Command processing system
fn process_commands_system() -> BoxedSystem {
    System::builder()
        .with_name("process_commands")
        .with_query(Query::new((pane_command_queue().as_mut())).entity(resources()))
        .with_cmd_mut()
        .with_world()
        .build(move |
          mut resources: EntityBorrow<'_, ComponentMut<VecDeque<Command>>>, cmdbuf: &mut CommandBuffer, world: &World,  | {
            let queue = resources.get().unwrap();

            println!("[System] Processing {} commands", queue.len());
            // Note: In a real system, we'd need a way to access world here
            // This is a limitation we'd need to work around
            for (index, cmd) in queue.drain(..).enumerate() {
                match cmd {
                    Command::CreatePaneWithDatasets { dataset_ids } => {
                        println!(
                            "[System] Processing CreatePaneWithDatasets command with {} datasets",
                            dataset_ids.len()
                        );

                        cmdbuf.defer(move |world| {

                        let pane_handle = create_pane_with_datasets(world, dataset_ids, 100 * (index as u32 + 1), 200);
                        println!("[System] Created pane: {:?}", pane_handle);
                        Ok(())
                        });
                    }
                    Command::DeletePane { pane } => {
                        println!("[System] Processing DeletePane command for {:?}", pane);
                        cmdbuf.despawn(pane.entity());
                    }
                }
            }
        })
        .boxed()
}

fn dump_subscriptions_by_dataset(world: &World) {
    // Print all datasets and their subscriptions
    println!("\n=== Dataset Subscriptions ===");

    let mut dataset_query = Query::new((entity_ids(), dataset::id()));
    let mut binding = dataset_query.borrow(&world);
    let datasets: Vec<_> = binding.iter().collect();

    for (entity, &dataset_id) in datasets {
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
    // Create a new flax world
    let mut world = World::new();

    // Create command queue entity
    Entity::builder()
        .set(pane_command_queue(), VecDeque::new())
        .append_to(&mut world, resources())
        .unwrap();

    println!("=== Command-Based Pane Creation Demo ===\n");

    // Enqueue commands instead of direct creation
    println!("Enqueueing commands...");

    // Helper to enqueue commands
    fn enqueue_command(world: &mut World, cmd: Command) {
        let mut queue = world.get_mut(resources(), pane_command_queue()).unwrap();
        queue.push_back(cmd);
    }

    let query = Query::new(pane_command_queue()).entity(resources());

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
    let mut command_exec_schedules = Schedule::builder()
        .with_system(process_commands_system())
        .build();

    command_exec_schedules.execute_par(&mut world);

    // Get created panes from the command system
    let mut pane_query = Query::new(entity_ids()).with(pane::width());
    let pane_handles: Vec<PaneHandle> = pane_query
        .borrow(&world)
        .iter()
        .map(|entity| PaneHandle::new(entity))
        .collect();

    let pane1 = pane_handles[0];
    let pane2 = pane_handles[1];
    let pane3 = pane_handles[2];

    // Print all panes
    println!("\n=== Panes ===");
    {
        let mut query = Query::new((entity_ids(), pane::width(), pane::height()));
        let mut binding = query.borrow(&world);
        let pane_entities: Vec<_> = binding.iter().collect();

        for (pane_entity, width, height) in pane_entities {
            let pane_handle = PaneHandle::new(pane_entity);
            println!("Pane Handle: {:?}", pane_handle);
            println!("  Width: {}, Height: {}", *width, *height);

            // Query relations: what datasets does this pane use?
            // Use relations_like to efficiently get all uses_dataset relations for this pane
            let mut this_pane_datasets = Vec::new();
            let mut relation_query =
                Query::new((pane::width(), relations_like(pane::uses_dataset)));
            if let Ok((width, relations)) = relation_query.borrow(&world).get(pane_entity) {
                println!("  Width: {}", *width);
                for (target, _) in relations {
                    this_pane_datasets.push(DatasetHandle::new(target));
                }
            }

            if !this_pane_datasets.is_empty() {
                println!(
                    "  Uses {} datasets: {:?}",
                    this_pane_datasets.len(),
                    this_pane_datasets
                );
            } else {
                println!("  Uses no datasets");
            }
        }
    }

    dump_subscriptions_by_dataset(&world);

    // Use command to delete pane 3
    println!("\n=== Demonstrating Command-Based Deletion ===");
    println!("Enqueueing delete command for pane 3...");
    enqueue_command(&mut world, Command::DeletePane { pane: pane3 });

    // Process the delete command
    println!("Executing command processing system...\n");
    command_exec_schedules.execute_par(&mut world);

    dump_subscriptions_by_dataset(&world);

    // Print world statistics
    println!("\n=== World Statistics ===");

    // Count entities with different components
    let pane_count = Query::new(pane::width()).borrow(&world).iter().count();
    println!("Entities with pane components: {}", pane_count);

    let dataset_count = Query::new(dataset::id()).borrow(&world).iter().count();
    println!("Entities with dataset_id component: {}", dataset_count);

    // Count relation instances
    let mut uses_relation_count = 0;
    let mut pane_query = Query::new(entity_ids()).with(pane::width());
    let pane_entities: Vec<_> = pane_query.borrow(&world).iter().collect();
    let mut dataset_query = Query::new(entity_ids()).with(dataset::id());
    let dataset_entities: Vec<_> = dataset_query.borrow(&world).iter().collect();

    for pane_entity in &pane_entities {
        for dataset_entity in &dataset_entities {
            if world.has(*pane_entity, pane::uses_dataset(*dataset_entity)) {
                uses_relation_count += 1;
            }
        }
    }

    let mut subscribed_relation_count = 0;
    for dataset_entity in &dataset_entities {
        for pane_entity in &pane_entities {
            if world.has(*dataset_entity, dataset::subscribed_by(*pane_entity)) {
                subscribed_relation_count += 1;
            }
        }
    }

    println!("Uses_dataset relation instances: {}", uses_relation_count);
    println!(
        "Subscribed_by relation instances: {}",
        subscribed_relation_count
    );

    // List all entities and their components - use a query to get all entities
    println!("\n=== All Entities ===");
    let all_entities: Vec<Entity> = Query::new(entity_ids()).borrow(&world).iter().collect();
    println!("Total entities: {}", all_entities.len());

    for entity in all_entities {
        print!("Entity {:?}: ", entity);

        let mut components = Vec::new();

        if world.has(entity, pane::width()) {
            components.push("pane::width");
        }
        if world.has(entity, pane::height()) {
            components.push("pane::height");
        }
        if world.has(entity, dataset::id()) {
            components.push("dataset::id");
        }
        // Check for relation participation
        let mut dataset_query = Query::new(entity_ids()).with(dataset::id());
        let dataset_entities: Vec<_> = dataset_query.borrow(&world).iter().collect();
        let mut pane_query = Query::new(entity_ids()).with(pane::width());
        let pane_entities: Vec<_> = pane_query.borrow(&world).iter().collect();

        // Check if this entity uses any datasets
        let mut uses_any_dataset = false;
        for dataset_entity in &dataset_entities {
            if world.has(entity, pane::uses_dataset(*dataset_entity)) {
                uses_any_dataset = true;
                break;
            }
        }
        if uses_any_dataset {
            components.push("uses_dataset");
        }

        // Check if this entity is subscribed by any panes
        let mut subscribed_by_any = false;
        for pane_entity in &pane_entities {
            if world.has(entity, dataset::subscribed_by(*pane_entity)) {
                subscribed_by_any = true;
                break;
            }
        }
        if subscribed_by_any {
            components.push("subscribed_by");
        }
        // Registry components removed

        println!("Components: {:?}", components);
    }

    // Show archetype information using queries
    println!("\n=== Archetype Analysis ===");

    // Query panes (entities with both width and height)
    let pane_count = Query::new((pane::width(), pane::height()))
        .borrow(&world)
        .iter()
        .count();
    println!("Pane archetype: {} entities", pane_count);

    // Query datasets (entities with dataset_id)
    let dataset_count = Query::new(dataset::id()).borrow(&world).iter().count();
    println!("Dataset archetype: {} entities", dataset_count);

    // No more registry entities

    // Demonstrate advanced queries
    println!("\n=== Query Examples ===");

    // Query all panes and their dimensions
    println!("All panes and their dimensions:");
    Query::new((pane::width(), pane::height()))
        .borrow(&world)
        .iter()
        .for_each(|(width, height)| {
            println!("  Pane: {}x{}", *width, *height);
        });

    // Query all datasets and show their IDs
    println!("All datasets:");
    Query::new(dataset::id())
        .borrow(&world)
        .iter()
        .for_each(|id| {
            println!("  Dataset: {:#?}", id);
        });

    // Demonstrate type safety - these would be compile errors:
    // let wrong_panes = get_panes_for_dataset(&world, pane1); // Error: expected DatasetHandle, found PaneHandle
    // let mixed_handles: Vec<Entity> = vec![pane1, dataset1]; // Error: can't mix handle types

    println!("\n=== Flax Example Complete ===");
    println!("This demonstrates Flax ECS functionality:");
    println!("- Component definition using component! macro");
    println!("- Entity creation with builder pattern");
    println!("- Query system with flexible component combinations");
    println!("- World introspection and archetype analysis");
    println!(
        "- TYPE-SAFE ENTITY HANDLES: PaneHandle and DatasetHandle prevent mixing entity types"
    );
    println!("- COMMAND SYSTEM: Queue-based command processing with systems");
}
