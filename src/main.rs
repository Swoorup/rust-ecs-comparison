use flax::*;
use std::collections::HashMap;
use colored::*;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{self, MatchingBracketValidator, Validator};
use rustyline::{Context, Helper};

component! {
    has_child(child): &'static str,
    last_modified: f64,
    health: i32,
}

#[derive(Debug, Clone)]
enum ChangeType {
    Added,
    Modified,
    Removed,
}

#[derive(Debug, Clone)]
struct EntityChange {
    entity: Entity,
    name: String,
    change_type: ChangeType,
    timestamp: f64,
}

struct ReplState {
    world: World,
    entity_names: HashMap<String, Entity>,
    changes: Vec<EntityChange>,
    last_dump_time: f64,
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
        std::borrow::Cow::Owned(format!("\x1b[1m{}\x1b[m", hint))
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
        let commands = vec![
            "add entity",
            "get",
            "set-relation child",
            "set health",
            "dump",
            "dump added",
            "dump modified", 
            "dump removed",
            "list",
            "help",
            "quit",
            "exit",
        ];

        let line_up_to_pos = &line[..pos];
        let parts: Vec<&str> = line_up_to_pos.split_whitespace().collect();
        
        let mut candidates = Vec::new();
        let start;

        if parts.is_empty() || (parts.len() == 1 && !line_up_to_pos.ends_with(' ')) {
            // Complete command names
            let prefix = parts.first().map_or("", |v| v);
            start = pos - prefix.len();
            
            for cmd in &commands {
                if cmd.starts_with(prefix) {
                    candidates.push(Pair {
                        display: cmd.to_string(),
                        replacement: cmd.to_string(),
                    });
                }
            }
        } else {
            // Complete entity names for relevant commands
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
                ["set-relation", "child", _, "parent", partial] if !line_up_to_pos.ends_with(' ') => {
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
                _ => {
                    start = pos;
                }
            }
        }

        Ok((start, candidates))
    }
}

impl ReplState {
    fn new() -> Self {
        Self {
            world: World::new(),
            entity_names: HashMap::new(),
            changes: Vec::new(),
            last_dump_time: 0.0,
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

        self.changes.push(EntityChange {
            entity,
            name: name.to_string(),
            change_type: ChangeType::Added,
            timestamp,
        });

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

        let is_new = self.world.get(entity, health()).is_err();

        self.world
            .set(entity, health(), health_value)
            .map_err(|e| format!("Failed to set health: {:?}", e))?;

        self.world.set(entity, last_modified(), timestamp).ok();

        self.changes.push(EntityChange {
            entity,
            name: name.to_string(),
            change_type: if is_new {
                ChangeType::Added
            } else {
                ChangeType::Modified
            },
            timestamp,
        });

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

        self.changes.push(EntityChange {
            entity: child,
            name: child_name.to_string(),
            change_type: ChangeType::Modified,
            timestamp,
        });

        self.changes.push(EntityChange {
            entity: parent,
            name: parent_name.to_string(),
            change_type: ChangeType::Modified,
            timestamp,
        });

        Ok(())
    }

    fn get_current_time(&self) -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }

    fn dump_changes(&mut self, filter: Option<&str>) {
        let current_time = self.get_current_time();

        let changes_to_dump: Vec<_> = self
            .changes
            .iter()
            .filter(|c| c.timestamp > self.last_dump_time)
            .filter(|c| match filter {
                Some("added") => matches!(c.change_type, ChangeType::Added),
                Some("modified") => matches!(c.change_type, ChangeType::Modified),
                Some("removed") => matches!(c.change_type, ChangeType::Removed),
                _ => true,
            })
            .cloned()
            .collect();

        if changes_to_dump.is_empty() {
            println!("{}", "No changes to display".yellow());
            return;
        }

        let title = match filter {
            Some("added") => "=== Added Entities ===".green().bold(),
            Some("modified") => "=== Modified Entities ===".blue().bold(),
            Some("removed") => "=== Removed Entities ===".red().bold(),
            _ => "=== All Changes ===".cyan().bold(),
        };

        println!("\n{}", title);

        for change in changes_to_dump {
            let (change_type_str, _color) = match change.change_type {
                ChangeType::Added => ("ADDED".green().bold(), "green"),
                ChangeType::Modified => ("MODIFIED".blue().bold(), "blue"),
                ChangeType::Removed => ("REMOVED".red().bold(), "red"),
            };

            println!(
                "  [{}] {} {} ({})",
                change_type_str, 
                "Entity".white(),
                format!("{:?}", change.entity).bright_magenta(),
                change.name.bright_cyan()
            );

            if let Ok(health_val) = self.world.get(change.entity, health()) {
                let health_color = if *health_val > 75 {
                    format!("{}", *health_val).green()
                } else if *health_val > 30 {
                    format!("{}", *health_val).yellow()
                } else {
                    format!("{}", *health_val).red()
                };
                println!("    {} {}", "Health:".bright_black(), health_color);
            }

            if let Ok(child_of_relations) = Query::new(relations_like(components::child_of))
                .with_relation(components::child_of)
                .borrow(&self.world)
                .get(change.entity)
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
                    println!("    {} {}", 
                        "Parents:".bright_black(), 
                        parents.join(", ").bright_yellow());
                }
            }

            if let Ok(has_child_relations) = Query::new(relations_like(has_child))
                .borrow(&self.world)
                .get(change.entity)
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
                    println!("    {} {}", 
                        "Children:".bright_black(), 
                        children.join(", ").bright_green());
                }
            }
        }

        self.last_dump_time = current_time;
        println!("{}\n", "========================".bright_black());
    }

    fn get_entity_info(&self, name: &str) -> Result<String, String> {
        let entity = self.get_entity(name)?;

        let mut info = String::new();
        info.push_str(&format!("{} {} ({})\n", 
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
            info.push_str(&format!("  {} {}\n", "Health:".bright_black(), health_color));
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
                info.push_str(&format!("  {} {}\n", 
                    "Parents:".bright_black(), 
                    parents.join(", ").bright_yellow()));
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
                info.push_str(&format!("  {} {}\n", 
                    "Children:".bright_black(), 
                    children.join(", ").bright_green()));
            }
        }

        Ok(info)
    }
}

fn print_help() {
    println!("{}", "Available commands:".cyan().bold());
    println!("  {} - Add a new entity with the given name", 
        "add entity [name]".green());
    println!("  {} - Get information about an entity", 
        "get [name]".green());
    println!("  {} - Create a parent-child relation", 
        "set-relation child [name] parent [name]".green());
    println!("  {} - Set health value for an entity", 
        "set health [name] [number]".green());
    println!("  {} - Show all recent changes", 
        "dump".green());
    println!("  {} - Show recently added entities", 
        "dump added".green());
    println!("  {} - Show recently modified entities", 
        "dump modified".green());
    println!("  {} - Show recently removed entities", 
        "dump removed".green());
    println!("  {} - List all entities", 
        "list".green());
    println!("  {} - Show this help message", 
        "help".green());
    println!("  {} - Exit the REPL", 
        "quit".green());
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
    let mut rl = Editor::new()?;
    rl.set_helper(Some(h));

    println!("{}", "╔═══════════════════════════╗".bright_magenta());
    println!("{}", "║     Flax ECS REPL v1.0   ║".bright_magenta().bold());
    println!("{}", "╚═══════════════════════════╝".bright_magenta());
    println!("{}\n", "Type 'help' for available commands".bright_black());
    println!("{}", "Tab completion is available for commands and entity names!".bright_cyan());

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
                    println!("{} Created entity '{}' with id {}", 
                        "✓".green().bold(),
                        name.bright_cyan(),
                        format!("{:?}", entity).bright_magenta());
                }
                Err(e) => println!("{} {}", "✗".red().bold(), e.red()),
            },
            ["get", name] => match state.get_entity_info(name) {
                Ok(info) => print!("{}", info),
                Err(e) => println!("{} {}", "✗".red().bold(), e.red()),
            },
            ["set-relation", "child", child_name, "parent", parent_name] => {
                match state.add_relation(child_name, parent_name) {
                    Ok(_) => {
                        println!("{} Created relation: {} {} {} {}", 
                            "✓".green().bold(),
                            child_name.bright_cyan(),
                            "is child of".white(),
                            parent_name.bright_yellow(),
                            "🔗".bright_blue());
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
                        println!("{} Set health of '{}' to {} {}", 
                            "✓".green().bold(),
                            name.bright_cyan(),
                            health_value.to_string().bright_green(),
                            health_icon);
                    }
                    Err(e) => println!("{} {}", "✗".red().bold(), e.red()),
                },
                Err(_) => println!("{} Invalid health value '{}', must be a number", 
                    "✗".red().bold(), 
                    number_str.red()),
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
                        println!("  {} {} ({})", 
                            "•".bright_blue(),
                            name.bright_cyan(),
                            format!("{:?}", entity).bright_magenta());
                    }
                }
            }
                    _ => {
                        println!("{} Unknown command: '{}'", 
                            "⚠".yellow().bold(), 
                            input.red());
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
