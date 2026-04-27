//! PardusDB - A single-file embedded vector database with SQL-like interface.

use std::io::{self, Write};
use std::path::PathBuf;

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

fn run_with_file(path: &str) {
    println!("=== PardusDB ===");
    println!("Opening database: {}", path);

    match Database::open(path) {
        Ok(_db) => {
            println!("Database opened successfully.\n");
        }
        Err(e) => println!("Error opening database: {}", e),
    }
}

fn run_repl() {
    print_welcome();

    let mut db = Database::in_memory();
    let mut current_file: Option<PathBuf> = None;

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
                }
                Err(e) => println!("Error opening: {}", e),
            }
            continue;
        }

        if cmd.starts_with("create ") {
            let path = cmd[7..].trim();
            // Create new database file
            match Database::open(path) {
                Ok(new_db) => {
                    db = new_db;
                    current_file = Some(PathBuf::from(path));
                    println!("Created and opened: {}", path);
                    println!("Now you can create tables with: CREATE TABLE ...");
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

