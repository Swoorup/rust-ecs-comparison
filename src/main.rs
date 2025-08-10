use colored::*;
use flax::system::BoxedSystem;
use flax::*;
use rustyline::Editor;
use rustyline::completion::{Completer, Pair};
use rustyline::config::{Config, EditMode};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{self, MatchingBracketValidator, Validator};
use rustyline::{Cmd, KeyEvent};
use rustyline::{Context, Helper};
use std::collections::HashMap;

component! {
    has_child(child): &'static str,
    last_modified: f64,
    health: i32,
}

struct ReplState {
    world: World,
    entity_names: HashMap<String, Entity>,
    // Systems for change detection
    added_system: BoxedSystem,
    modified_system: BoxedSystem,
    removed_system: BoxedSystem,
}

struct MyHelper {
    completer: MyCompleter,
    highlighter: MatchingBracketHighlighter,
    validator: MatchingBracketValidator,
    hinter: HistoryHinter,
    colored_prompt: String,
}

impl Completer for MyHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for MyHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        // First try our custom completion hints
        if let Ok((start, completions)) = self.completer.complete(line, pos, ctx) {
            if !completions.is_empty() && start < pos {
                let input_prefix = &line[start..pos];
                let first_completion = &completions[0].replacement;

                if first_completion.len() > input_prefix.len()
                    && first_completion.starts_with(input_prefix)
                {
                    return Some(first_completion[input_prefix.len()..].to_string());
                }
            }
        }

        // Fall back to history hints
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> std::borrow::Cow<'b, str> {
        if default {
            std::borrow::Cow::Borrowed(&self.colored_prompt)
        } else {
            std::borrow::Cow::Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        // Use dim/gray color (ANSI code 90) for completion hints
        std::borrow::Cow::Owned(format!("\x1b[90m{}\x1b[0m", hint))
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> std::borrow::Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, forced: bool) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }
}

impl Validator for MyHelper {
    fn validate(
        &self,
        ctx: &mut validate::ValidationContext,
    ) -> rustyline::Result<validate::ValidationResult> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

impl Helper for MyHelper {}

struct MyCompleter {
    entity_names: Vec<String>,
}

impl MyCompleter {
    fn new() -> Self {
        Self {
            entity_names: Vec::new(),
        }
    }

    fn update_entities(&mut self, entities: &HashMap<String, Entity>) {
        self.entity_names = entities.keys().cloned().collect();
        self.entity_names.sort();
    }
}

impl Completer for MyCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let base_commands = vec![
            "add entity",
            "get",
            "set-relation child",
            "set health",
            "remove",
            "dump",
            "list",
            "help",
            "quit",
            "exit",
        ];

        let dump_subcommands = vec!["dump", "dump added", "dump modified", "dump removed"];

        let line_up_to_pos = &line[..pos];
        let parts: Vec<&str> = line_up_to_pos.split_whitespace().collect();

        let mut candidates = Vec::new();
        let mut start = pos;

        if parts.is_empty() || (parts.len() == 1 && !line_up_to_pos.ends_with(' ')) {
            // Complete command names
            let prefix = parts.first().map_or("", |v| v);
            start = pos - prefix.len();

            // Include base commands and dump sub-commands in initial completion
            let all_commands = [&base_commands[..], &dump_subcommands[..]].concat();
            for cmd in &all_commands {
                if cmd.starts_with(prefix) {
                    candidates.push(Pair {
                        display: cmd.to_string(),
                        replacement: cmd.to_string(),
                    });
                }
            }
        } else if parts.len() == 1 && line_up_to_pos.ends_with(' ') {
            // Handle completions after complete commands (like "dump ")
            match parts[0] {
                "dump" => {
                    start = pos;
                    for subcmd in &["added", "modified", "removed"] {
                        candidates.push(Pair {
                            display: subcmd.to_string(),
                            replacement: subcmd.to_string(),
                        });
                    }
                }
                "set-relation" => {
                    start = pos;
                    candidates.push(Pair {
                        display: "child".to_string(),
                        replacement: "child".to_string(),
                    });
                }
                "add" => {
                    start = pos;
                    candidates.push(Pair {
                        display: "entity".to_string(),
                        replacement: "entity".to_string(),
                    });
                }
                _ => {}
            }
        } else if parts.len() == 2 && !line_up_to_pos.ends_with(' ') {
            // Handle partial completions for second word
            match parts[0] {
                "dump" => {
                    let partial = parts[1];
                    start = pos - partial.len();
                    for subcmd in &["added", "modified", "removed"] {
                        if subcmd.starts_with(partial) {
                            candidates.push(Pair {
                                display: subcmd.to_string(),
                                replacement: subcmd.to_string(),
                            });
                        }
                    }
                }
                _ => {
                    // Fall through to existing entity completion logic below
                }
            }
        }

        // Handle entity name completions for commands that expect entity names
        if candidates.is_empty() {
            match parts.as_slice() {
                ["get", partial] if !line_up_to_pos.ends_with(' ') => {
                    start = pos - partial.len();
                    for entity in &self.entity_names {
                        if entity.starts_with(partial) {
                            candidates.push(Pair {
                                display: entity.clone(),
                                replacement: entity.clone(),
                            });
                        }
                    }
                }
                ["set", "health", partial] if !line_up_to_pos.ends_with(' ') => {
                    start = pos - partial.len();
                    for entity in &self.entity_names {
                        if entity.starts_with(partial) {
                            candidates.push(Pair {
                                display: entity.clone(),
                                replacement: entity.clone(),
                            });
                        }
                    }
                }
                ["remove", partial] if !line_up_to_pos.ends_with(' ') => {
                    start = pos - partial.len();
                    for entity in &self.entity_names {
                        if entity.starts_with(partial) {
                            candidates.push(Pair {
                                display: entity.clone(),
                                replacement: entity.clone(),
                            });
                        }
                    }
                }
                ["set-relation", "child", partial] if !line_up_to_pos.ends_with(' ') => {
                    start = pos - partial.len();
                    for entity in &self.entity_names {
                        if entity.starts_with(partial) {
                            candidates.push(Pair {
                                display: entity.clone(),
                                replacement: entity.clone(),
                            });
                        }
                    }
                }
                ["set-relation", "child", _, "parent", partial]
                    if !line_up_to_pos.ends_with(' ') =>
                {
                    start = pos - partial.len();
                    for entity in &self.entity_names {
                        if entity.starts_with(partial) {
                            candidates.push(Pair {
                                display: entity.clone(),
                                replacement: entity.clone(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        Ok((start, candidates))
    }
}

impl ReplState {
    fn new() -> Self {
        use flax::filter::ChangeFilter;
        use flax::query::QueryBorrow;

        // Create systems for change detection using the proper Flax System API
        let added_system = System::builder()
            .with_name("added_components")
            .with_query(Query::new((entity_ids(), components::name().added())))
            .with_query(Query::new((
                entity_ids(),
                components::name(),
                health().added(),
            )))
            .build(
                |mut name_query: QueryBorrow<(EntityIds, ChangeFilter<String>)>,
                 mut health_query: QueryBorrow<(
                    EntityIds,
                    flax::Component<String>,
                    ChangeFilter<i32>,
                )>| {
                    let mut found_changes = false;

                    // Query for newly added name components
                    for (entity, name) in name_query.iter() {
                        found_changes = true;
                        println!(
                            "  [{}] {} {} ({})",
                            "ADDED".green().bold(),
                            "Entity".white(),
                            format!("{:?}", entity).bright_magenta(),
                            name.bright_cyan()
                        );
                    }

                    // Query for newly added health components
                    for (entity, name, health_val) in health_query.iter() {
                        found_changes = true;
                        let health_color = if *health_val > 75 {
                            format!("{}", *health_val).green()
                        } else if *health_val > 30 {
                            format!("{}", *health_val).yellow()
                        } else {
                            format!("{}", *health_val).red()
                        };
                        println!(
                            "  [{}] {} {} ({}) - Health: {}",
                            "ADDED HEALTH".green().bold(),
                            "Entity".white(),
                            format!("{:?}", entity).bright_magenta(),
                            name.bright_cyan(),
                            health_color
                        );
                    }

                    if !found_changes {
                        println!("    {}", "No added components to display".yellow());
                    }
                    () // Explicitly return ()
                },
            )
            .boxed();

        let modified_system = System::builder()
            .with_name("modified_components")
            .with_query(Query::new((
                entity_ids(),
                components::name(),
                health().modified(),
            )))
            .with_query(Query::new((
                entity_ids(),
                components::name(),
                last_modified().modified(),
            )))
            .build(
                |mut health_query: QueryBorrow<(
                    EntityIds,
                    flax::Component<String>,
                    ChangeFilter<i32>,
                )>,
                 mut modified_query: QueryBorrow<(
                    EntityIds,
                    flax::Component<String>,
                    ChangeFilter<f64>,
                )>| {
                    let mut found_changes = false;

                    // Query for modified health components
                    for (entity, name, health_val) in health_query.iter() {
                        found_changes = true;
                        let health_color = if *health_val > 75 {
                            format!("{}", *health_val).green()
                        } else if *health_val > 30 {
                            format!("{}", *health_val).yellow()
                        } else {
                            format!("{}", *health_val).red()
                        };
                        println!(
                            "  [{}] {} {} ({}) - Health: {}",
                            "MODIFIED HEALTH".blue().bold(),
                            "Entity".white(),
                            format!("{:?}", entity).bright_magenta(),
                            name.bright_cyan(),
                            health_color
                        );
                    }

                    // Query for general modifications via last_modified
                    for (entity, name, _timestamp) in modified_query.iter() {
                        found_changes = true;
                        println!(
                            "  [{}] {} {} ({})",
                            "MODIFIED".blue().bold(),
                            "Entity".white(),
                            format!("{:?}", entity).bright_magenta(),
                            name.bright_cyan()
                        );
                    }

                    if !found_changes {
                        println!("    {}", "No modified components to display".yellow());
                    }
                    () // Explicitly return ()
                },
            )
            .boxed();

        let removed_system = System::builder()
            .with_name("removed_components")
            .build(|| {
                println!(
                    "    {}",
                    "Note: Removed component tracking not fully implemented yet".yellow()
                );
                () // Explicitly return ()
            })
            .boxed();

        Self {
            world: World::new(),
            entity_names: HashMap::new(),
            added_system,
            modified_system,
            removed_system,
        }
    }

    fn add_entity(&mut self, name: &str) -> Result<Entity, String> {
        if self.entity_names.contains_key(name) {
            return Err(format!("Entity '{}' already exists", name));
        }

        let timestamp = self.get_current_time();
        let entity = Entity::builder()
            .set(components::name(), name.to_string())
            .set(last_modified(), timestamp)
            .spawn(&mut self.world);

        self.entity_names.insert(name.to_string(), entity);

        Ok(entity)
    }

    fn get_entity(&self, name: &str) -> Result<Entity, String> {
        self.entity_names
            .get(name)
            .copied()
            .ok_or_else(|| format!("Entity '{}' not found", name))
    }

    fn set_health(&mut self, name: &str, health_value: i32) -> Result<(), String> {
        let entity = self.get_entity(name)?;
        let timestamp = self.get_current_time();

        self.world
            .set(entity, health(), health_value)
            .map_err(|e| format!("Failed to set health: {:?}", e))?;

        self.world.set(entity, last_modified(), timestamp).ok();

        Ok(())
    }

    fn add_relation(&mut self, child_name: &str, parent_name: &str) -> Result<(), String> {
        let child = self.get_entity(child_name)?;
        let parent = self.get_entity(parent_name)?;
        let timestamp = self.get_current_time();

        self.world
            .set(child, components::child_of(parent), ())
            .map_err(|e| format!("Failed to set child_of relation: {:?}", e))?;

        self.world
            .set(parent, has_child(child), "has_child")
            .map_err(|e| format!("Failed to set has_child relation: {:?}", e))?;

        self.world.set(child, last_modified(), timestamp).ok();
        self.world.set(parent, last_modified(), timestamp).ok();

        Ok(())
    }

    fn remove_entity(&mut self, name: &str) -> Result<(), String> {
        let entity = self.get_entity(name)?;

        // Remove the entity from the world (this will automatically clean up all components and relations)
        self.world
            .despawn(entity)
            .map_err(|e| format!("Failed to remove entity: {:?}", e))?;

        // Remove from our name lookup
        self.entity_names.remove(name);

        Ok(())
    }

    fn get_current_time(&self) -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }

    fn dump_changes(&mut self, filter: Option<&str>) {
        let title = match filter {
            Some("added") => "=== Added Components ===".green().bold(),
            Some("modified") => "=== Modified Components ===".blue().bold(),
            Some("removed") => "=== Removed Components ===".red().bold(),
            _ => "=== All Changes ===".cyan().bold(),
        };

        println!("\n{}", title);

        match filter {
            Some("added") => {
                self.added_system.run(&mut self.world).unwrap();
            }
            Some("modified") => {
                self.modified_system.run(&mut self.world).unwrap();
            }
            Some("removed") => {
                self.removed_system.run(&mut self.world).unwrap();
            }
            _ => {
                self.show_relations();
            }
        }

        println!("{}\n", "========================".bright_black());
    }

    fn show_relations(&self) {
        // Show relations for entities that were modified via last_modified changes
        Query::new((entity_ids(), components::name()))
            .borrow(&self.world)
            .for_each(|(entity, name)| {
                // First print the entity
                println!(
                    "  {} {} ({})",
                    "Entity".white(),
                    name.bright_cyan(),
                    format!("{:?}", entity).bright_magenta()
                );
                // Then show its relations
                self.display_entity_relations(entity);
            });
    }

    fn display_entity_relations(&self, entity: Entity) {
        // Show parent relationships
        if let Ok(child_of_relations) = Query::new(relations_like(components::child_of))
            .with_relation(components::child_of)
            .borrow(&self.world)
            .get(entity)
        {
            let parents: Vec<String> = child_of_relations
                .map(|(parent, _)| {
                    self.world
                        .get(parent, components::name())
                        .map(|n| n.clone())
                        .unwrap_or_else(|_| format!("{:?}", parent))
                })
                .collect();

            if !parents.is_empty() {
                println!(
                    "      {} {}",
                    "Parents:".bright_black(),
                    parents.join(", ").bright_yellow()
                );
            }
        }

        // Show child relationships
        if let Ok(has_child_relations) = Query::new(relations_like(has_child))
            .borrow(&self.world)
            .get(entity)
        {
            let children: Vec<String> = has_child_relations
                .map(|(child, _): (Entity, &&str)| {
                    self.world
                        .get(child, components::name())
                        .map(|n| n.clone())
                        .unwrap_or_else(|_| format!("{:?}", child))
                })
                .collect();

            if !children.is_empty() {
                println!(
                    "      {} {}",
                    "Children:".bright_black(),
                    children.join(", ").bright_green()
                );
            }
        }
    }

    fn get_entity_info(&self, name: &str) -> Result<String, String> {
        let entity = self.get_entity(name)?;

        let mut info = String::new();
        info.push_str(&format!(
            "{} {} ({})\n",
            "Entity:".white().bold(),
            name.bright_cyan().bold(),
            format!("{:?}", entity).bright_magenta()
        ));

        if let Ok(health_val) = self.world.get(entity, health()) {
            let health_color = if *health_val > 75 {
                format!("{}", *health_val).green()
            } else if *health_val > 30 {
                format!("{}", *health_val).yellow()
            } else {
                format!("{}", *health_val).red()
            };
            info.push_str(&format!(
                "  {} {}\n",
                "Health:".bright_black(),
                health_color
            ));
        }

        if let Ok(child_of_relations) = Query::new(relations_like(components::child_of))
            .with_relation(components::child_of)
            .borrow(&self.world)
            .get(entity)
        {
            let parents: Vec<String> = child_of_relations
                .map(|(parent, _)| {
                    self.world
                        .get(parent, components::name())
                        .map(|n| n.clone())
                        .unwrap_or_else(|_| format!("{:?}", parent))
                })
                .collect();

            if !parents.is_empty() {
                info.push_str(&format!(
                    "  {} {}\n",
                    "Parents:".bright_black(),
                    parents.join(", ").bright_yellow()
                ));
            }
        }

        if let Ok(has_child_relations) = Query::new(relations_like(has_child))
            .borrow(&self.world)
            .get(entity)
        {
            let children: Vec<String> = has_child_relations
                .map(|(child, _): (Entity, &&str)| {
                    self.world
                        .get(child, components::name())
                        .map(|n| n.clone())
                        .unwrap_or_else(|_| format!("{:?}", child))
                })
                .collect();

            if !children.is_empty() {
                info.push_str(&format!(
                    "  {} {}\n",
                    "Children:".bright_black(),
                    children.join(", ").bright_green()
                ));
            }
        }

        Ok(info)
    }
}

fn print_help() {
    println!("{}", "Available commands:".cyan().bold());
    println!(
        "  {} - Add a new entity with the given name",
        "add entity [name]".green()
    );
    println!(
        "  {} - Get information about an entity",
        "get [name]".green()
    );
    println!(
        "  {} - Create a parent-child relation",
        "set-relation child [name] parent [name]".green()
    );
    println!(
        "  {} - Set health value for an entity",
        "set health [name] [number]".green()
    );
    println!("  {} - Remove an entity", "remove [name]".green());
    println!("  {} - Show all recent changes", "dump".green());
    println!("  {} - Show recently added entities", "dump added".green());
    println!(
        "  {} - Show recently modified entities",
        "dump modified".green()
    );
    println!(
        "  {} - Show recently removed entities",
        "dump removed".green()
    );
    println!("  {} - List all entities", "list".green());
    println!("  {} - Show this help message", "help".green());
    println!("  {} - Exit the REPL", "quit".green());
}

fn main() -> rustyline::Result<()> {
    let mut state = ReplState::new();
    let h = MyHelper {
        completer: MyCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter::new(),
        validator: MatchingBracketValidator::new(),
        colored_prompt: format!("{} ", "►".bright_green().bold()),
    };

    let config = Config::builder()
        .edit_mode(EditMode::Emacs)
        .completion_type(rustyline::config::CompletionType::Circular)
        .auto_add_history(true)
        .build();

    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(h));

    // Bind Command-E (Alt-E on some systems) to complete and move to end of line
    rl.bind_sequence(KeyEvent::alt('e'), Cmd::CompleteHint);

    // Also bind it to Ctrl-E for compatibility
    rl.bind_sequence(KeyEvent::ctrl('E'), Cmd::CompleteHint);

    println!("{}", "╔═══════════════════════════╗".bright_magenta());
    println!("{}", "║     Flax ECS REPL v1.0   ║".bright_magenta().bold());
    println!("{}", "╚═══════════════════════════╝".bright_magenta());
    println!("{}\n", "Type 'help' for available commands".bright_black());
    println!(
        "{}",
        "Tab completion is available for commands and entity names!".bright_cyan()
    );
    println!(
        "{}",
        "Use Tab to cycle completions, Cmd-E/Ctrl-E for hint completion".bright_black()
    );

    loop {
        // Update entity completion list
        if let Some(helper) = rl.helper_mut() {
            helper.completer.update_entities(&state.entity_names);
        }

        let readline = rl.readline("► ");
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                rl.add_history_entry(input).ok();

                let parts: Vec<&str> = input.split_whitespace().collect();

                match parts.as_slice() {
                    ["quit"] | ["exit"] => {
                        println!("{}", "👋 Goodbye!".bright_cyan());
                        break;
                    }
                    ["help"] => {
                        print_help();
                    }
                    ["add", "entity", name] => match state.add_entity(name) {
                        Ok(entity) => {
                            println!(
                                "{} Created entity '{}' with id {}",
                                "✓".green().bold(),
                                name.bright_cyan(),
                                format!("{:?}", entity).bright_magenta()
                            );
                        }
                        Err(e) => println!("{} {}", "✗".red().bold(), e.red()),
                    },
                    ["get", name] => match state.get_entity_info(name) {
                        Ok(info) => print!("{}", info),
                        Err(e) => println!("{} {}", "✗".red().bold(), e.red()),
                    },
                    ["remove", name] => match state.remove_entity(name) {
                        Ok(_) => {
                            println!(
                                "{} Removed entity '{}'",
                                "✓".green().bold(),
                                name.bright_cyan()
                            );
                        }
                        Err(e) => println!("{} {}", "✗".red().bold(), e.red()),
                    },
                    ["set-relation", "child", child_name, "parent", parent_name] => {
                        match state.add_relation(child_name, parent_name) {
                            Ok(_) => {
                                println!(
                                    "{} Created relation: {} {} {} {}",
                                    "✓".green().bold(),
                                    child_name.bright_cyan(),
                                    "is child of".white(),
                                    parent_name.bright_yellow(),
                                    "🔗".bright_blue()
                                );
                            }
                            Err(e) => println!("{} {}", "✗".red().bold(), e.red()),
                        }
                    }
                    ["set", "health", name, number_str] => match number_str.parse::<i32>() {
                        Ok(health_value) => match state.set_health(name, health_value) {
                            Ok(_) => {
                                let health_icon = if health_value > 75 {
                                    "💚"
                                } else if health_value > 30 {
                                    "💛"
                                } else {
                                    "❤️"
                                };
                                println!(
                                    "{} Set health of '{}' to {} {}",
                                    "✓".green().bold(),
                                    name.bright_cyan(),
                                    health_value.to_string().bright_green(),
                                    health_icon
                                );
                            }
                            Err(e) => println!("{} {}", "✗".red().bold(), e.red()),
                        },
                        Err(_) => println!(
                            "{} Invalid health value '{}', must be a number",
                            "✗".red().bold(),
                            number_str.red()
                        ),
                    },
                    ["dump"] => {
                        state.dump_changes(None);
                    }
                    ["dump", "added"] => {
                        state.dump_changes(Some("added"));
                    }
                    ["dump", "modified"] => {
                        state.dump_changes(Some("modified"));
                    }
                    ["dump", "removed"] => {
                        state.dump_changes(Some("removed"));
                    }
                    ["list"] => {
                        if state.entity_names.is_empty() {
                            println!("{}", "No entities created yet".yellow());
                        } else {
                            println!("{}", "📋 Entities:".cyan().bold());
                            for (name, entity) in &state.entity_names {
                                println!(
                                    "  {} {} ({})",
                                    "•".bright_blue(),
                                    name.bright_cyan(),
                                    format!("{:?}", entity).bright_magenta()
                                );
                            }
                        }
                    }
                    _ => {
                        println!("{} Unknown command: '{}'", "⚠".yellow().bold(), input.red());
                        println!("{}", "Type 'help' for available commands".bright_black());
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("{} Error: {:?}", "✗".red().bold(), err);
                break;
            }
        }
    }
    Ok(())
}
