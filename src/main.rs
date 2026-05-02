//! PardusDB - A single-file embedded vector database with SQL-like interface.

use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use pardusdb::{Database, ExecuteResult};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        let path = &args[1];
        run_with_file(path);
    } else {
        run_repl();
    }
}

fn find_project_db() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let db_path = cwd.join("database.pardus");
    if db_path.exists() {
        Some(db_path)
    } else {
        None
    }
}

fn ensure_marker(dir: &Path, db_path: &Path) {
    let marker = dir.join(".database.pardusdb");
    let now = chrono::Utc::now().to_rfc3339();

    if marker.exists() {
        if let Ok(content) = fs::read_to_string(&marker) {
            if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = json.as_object_mut() {
                    obj.insert("last_opened".to_string(), serde_json::Value::String(now));
                    let _ = fs::write(&marker, serde_json::to_string_pretty(obj).unwrap_or_default());
                }
            }
        }
    } else {
        let content = format!(
            r#"{{"version":1,"created_at":"{}","last_opened":"{}"}}"#,
            now, now
        );
        let _ = fs::write(&marker, content);

        if dir.join(".git").exists() {
            let gitignore = dir.join(".gitignore");
            let current = fs::read_to_string(&gitignore).unwrap_or_default();
            if !current.contains("*.pardus") {
                let _ = fs::write(&gitignore, format!("{}\n*.pardus\n", current));
            }
        }
    }
}

fn run_with_file(path: &str) {
    println!("=== PardusDB ===");
    println!("Opening database: {}", path);

    match Database::open(path) {
        Ok(mut db) => {
            println!("Database opened successfully.\n");

            if let Some(parent) = Path::new(path).parent() {
                ensure_marker(parent, Path::new(path));
            }

            let stdin = io::stdin();
            let mut lines = BufReader::new(stdin.lock()).lines().map(|r| r.unwrap_or_default()).peekable();

            if lines.peek().is_none() {
                if let Err(e) = db.save() {
                    println!("Error saving database: {}", e);
                } else {
                    println!("\nDatabase saved to: {}", path);
                }
                return;
            }

            for line in lines {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }

                let cmd = if input.starts_with('.') {
                    &input[1..]
                } else {
                    input
                };

                match cmd {
                    "quit" | "exit" | "q" => {
                        if let Err(e) = db.save() {
                            println!("Error saving: {}", e);
                        } else {
                            println!("Saved to: {}", path);
                        }
                        break;
                    }
                    "save" => {
                        if let Err(e) = db.save() {
                            println!("Error saving: {}", e);
                        } else {
                            println!("Saved to: {}", path);
                        }
                    }
                    "tables" => {
                        match db.execute("SHOW TABLES;") {
                            Ok(result) => println!("{}", result),
                            Err(e) => println!("Error: {}", e),
                        }
                    }
                    "help" | "?" => {
                        print_help();
                    }
                    "clear" | "cls" => {
                        print!("\x1B[2J\x1B[1;1H");
                    }
                    _ if cmd.starts_with("open ") || cmd.starts_with("create ") => {
                        println!("Note: Database already open. Use '.quit' to close and open a different one.");
                    }
                    _ => {
                        if input.starts_with('.') {
                            println!("Unknown command: {}", input);
                            println!("Type 'help' for available commands.");
                        } else {
                            match db.execute(input) {
                                Ok(result) => println!("{}", result),
                                Err(e) => println!("Error: {}", e),
                            }
                        }
                    }
                }
            }
        }
        Err(e) => println!("Error opening database: {}", e),
    }
}

fn run_repl() {
    print_welcome();

    let mut db = Database::in_memory();
    let mut current_file: Option<PathBuf> = None;

    if let Some(project_db) = find_project_db() {
        match Database::open(project_db.as_path()) {
            Ok(new_db) => {
                db = new_db;
                current_file = Some(project_db.clone());
                if let Some(parent) = project_db.parent() {
                    ensure_marker(parent, &project_db);
                }
                println!("Opened project database: {}\n", project_db.display());
            }
            Err(e) => {
                println!("Note: Project database found at {} but could not be opened: {}\n", project_db.display(), e);
            }
        }
    } else {
        println!("No project database found. Using in-memory database.");
        println!("To use a project DB: create .pardus in your project directory, or use .open <file>.\n");
    }

    loop {
        if current_file.is_some() {
            print!("pardusdb [{}]> ", current_file.as_ref().unwrap().display());
        } else {
            print!("pardusdb [memory]> ");
        }
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() { continue; }

        // Handle both "help" and ".help", "quit" and ".quit", etc.
        let cmd = if input.starts_with('.') {
            &input[1..]
        } else {
            input
        };

        // Check for meta commands
        match cmd {
            "help" | "?" => {
                print_help();
                continue;
            }
            "quit" | "exit" | "q" => {
                // Auto-save if file is open
                if let Some(ref path) = current_file {
                    match db.save() {
                        Ok(()) => println!("Saved to: {}", path.display()),
                        Err(e) => println!("Error saving: {}", e),
                    }
                }
                break;
            }
            "tables" => {
                match db.execute("SHOW TABLES;") {
                    Ok(result) => println!("{}", result),
                    Err(e) => println!("Error: {}", e),
                }
                continue;
            }
            "save" => {
                if let Some(ref path) = current_file {
                    match db.save() {
                        Ok(()) => println!("Saved to: {}", path.display()),
                        Err(e) => println!("Error: {}", e),
                    }
                } else {
                    println!("No file associated. Use: .open <file> or .create <file>");
                }
                continue;
            }
            "clear" | "cls" => {
                print!("\x1B[2J\x1B[1;1H");  // ANSI clear screen
                continue;
            }
            _ => {}
        }

        // Handle commands with arguments
        if cmd.starts_with("open ") {
            let path = cmd[5..].trim();
            match Database::open(path) {
                Ok(new_db) => {
                    db = new_db;
                    current_file = Some(PathBuf::from(path));
                    println!("Opened: {}", path);
                    if let Some(parent) = Path::new(path).parent() {
                        ensure_marker(parent, Path::new(path));
                    }
                }
                Err(e) => println!("Error opening: {}", e),
            }
            continue;
        }

        if cmd.starts_with("create ") {
            let path = cmd[7..].trim();
            match Database::open(path) {
                Ok(new_db) => {
                    db = new_db;
                    current_file = Some(PathBuf::from(path));
                    println!("Created and opened: {}", path);
                    println!("Now you can create tables with: CREATE TABLE ...");
                    if let Some(parent) = Path::new(path).parent() {
                        ensure_marker(parent, Path::new(path));
                    }
                }
                Err(e) => println!("Error creating: {}", e),
            }
            continue;
        }

        // If input started with . but wasn't recognized
        if input.starts_with('.') {
            println!("Unknown command: {}", input);
            println!("Type 'help' for available commands.");
            continue;
        }

        // Execute SQL
        match db.execute(input) {
            Ok(result) => println!("{}", result),
            Err(e) => println!("Error: {}", e),
        }
    }
    println!("Goodbye!");
}

fn print_welcome() {
    println!(r#"
╔═══════════════════════════════════════════════════════════════╗
║                        PardusDB REPL                          ║
║              Vector Database with SQL Interface               ║
╚═══════════════════════════════════════════════════════════════╝

Quick Start:
  .create mydb.pardus     Create a new database file
  .open mydb.pardus       Open an existing database

  CREATE TABLE docs (embedding VECTOR(768), content TEXT);
  INSERT INTO docs (embedding, content) VALUES ([0.1, 0.2, ...], 'text');
  SELECT * FROM docs WHERE embedding SIMILARITY [0.1, ...] LIMIT 5;

Type 'help' for all commands, 'quit' to exit.

"#);
}

fn print_help() {
    println!(r#"
┌─────────────────────────────────────────────────────────────────┐
│                     PardusDB Commands                           │
├─────────────────────────────────────────────────────────────────┤
│ DATABASE FILES                                                  │
│   .create <file>    Create a new database file                 │
│   .open <file>      Open an existing database                  │
│   .save             Save current database to file              │
│                                                                  │
│ INFORMATION                                                     │
│   .tables           List all tables                            │
│   help              Show this help message                     │
│                                                                  │
│ OTHER                                                           │
│   .clear            Clear screen                               │
│   quit / exit       Exit REPL (auto-saves if file open)        │
├─────────────────────────────────────────────────────────────────┤
│ SQL COMMANDS                                                    │
├─────────────────────────────────────────────────────────────────┤
│ CREATE TABLE <name> (<column> <type>, ...);                    │
│   Types: VECTOR(n), TEXT, INTEGER, FLOAT, BOOLEAN              │
│                                                                  │
│ INSERT INTO <table> (<columns>) VALUES (<values>);             │
│   Values: 'text', 123, 1.5, [0.1, 0.2, ...], true, null        │
│                                                                  │
│ SELECT * FROM <table> [WHERE ...] [LIMIT n];                   │
│ SELECT * FROM <table> WHERE <col> SIMILARITY [vec] LIMIT n;    │
│                                                                  │
│ UPDATE <table> SET <col> = <val> [WHERE ...];                  │
│ DELETE FROM <table> [WHERE ...];                               │
│ SHOW TABLES;                                                    │
│ DROP TABLE <name>;                                              │
├─────────────────────────────────────────────────────────────────┤
│ EXAMPLE WORKFLOW                                                │
├─────────────────────────────────────────────────────────────────┤
│   .create mydb.pardus                                           │
│   CREATE TABLE docs (embedding VECTOR(768), content TEXT);     │
│   INSERT INTO docs (embedding, content)                        │
│       VALUES ([0.1, 0.2, 0.3, ...], 'Hello World');            │
│   SELECT * FROM docs WHERE embedding                           │
│       SIMILARITY [0.1, 0.2, 0.3, ...] LIMIT 5;                 │
│   quit                                                          │
└─────────────────────────────────────────────────────────────────┘
"#);
}


