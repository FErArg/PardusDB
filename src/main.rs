//! PardusDB - A single-file embedded vector database with SQL-like interface.

use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

use pardusdb::{Database, ExecuteResult};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        let arg = &args[1];
        if arg == "--stats" {
            show_stats();
        } else {
            run_with_file(arg);
        }
    } else {
        run_repl();
    }
}

fn show_stats() {
    let stats_path = dirs::home_dir()
        .map(|h| h.join(".pardus").join("mcp_stats.json"))
        .unwrap_or_else(|| PathBuf::from(".pardus/mcp_stats.json"));

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║              PARDUSDB TOKEN DASHBOARD                        ║");
    println!("╠═══════════════════════════════════════════════════════════════╣");

    if stats_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&stats_path) {
            if let Ok(stats) = serde_json::from_str::<serde_json::Value>(&content) {
                let model = stats["config"]["model"].as_str().unwrap_or("unknown");
                let provider = stats["config"]["provider"].as_str().unwrap_or("unknown");
                let ctx = stats["config"]["context_window"].as_i64().unwrap_or(0);

                println!("║  MODEL: {}                                ║", truncate(model, 54));
                println!("║  Provider: {} | Context: {:>12} tokens   ║", provider, ctx);
                println!("╠═══════════════════════════════════════════════════════════════╣");
                println!("║  SESIÓN                                    TOTAL           ║");
                println!("║  ─────────────────────────────────────────────────────────   ║");

                let sq = stats["session"]["queries"].as_i64().unwrap_or(0);
                let st = stats["session"]["tokens_sent"].as_i64().unwrap_or(0);
                let sfull = stats["session"]["tokens_if_full"].as_i64().unwrap_or(0);
                let ssave = sfull - st;
                let spct = if sfull > 0 { (ssave as f64 / sfull as f64 * 100.0) as i64 } else { 0 };

                let tq = stats["total"]["queries"].as_i64().unwrap_or(0);
                let tt = stats["total"]["tokens_sent"].as_i64().unwrap_or(0);
                let tfull = stats["total"]["tokens_if_full"].as_i64().unwrap_or(0);
                let tsave = tfull - tt;
                let tpct = if tfull > 0 { (tsave as f64 / tfull as f64 * 100.0) as i64 } else { 0 };

                println!("║  Queries: {:>6}                    Total: {:>10}         ║", sq, tq);
                println!("║  Tokens: {:>6}                     Total: {:>10}         ║", format_num(st), format_num(tt));
                println!("║  Savings: {:>5} ({:>3}%)               Savings: {:>9} ({:>3}%) ║", format_num(ssave), spct, format_num(tsave), tpct);
                println!("╚═══════════════════════════════════════════════════════════════╝");
                return;
            }
        }
        println!("║  Error reading stats file                                  ║");
    } else {
        println!("║  No stats file found                                       ║");
        println!("║  Run some queries first to see statistics here               ║");
    }
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("\nStats file: {}", stats_path.display());
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max-3])
    } else {
        s.to_string()
    }
}

fn format_num(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn run_with_file(path: &str) {
    println!("=== PardusDB ===");
    println!("Opening database: {}", path);

    match Database::open(path) {
        Ok(mut db) => {
            println!("Database opened successfully.\n");

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

            let mut saw_quit = false;
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
                        saw_quit = true;
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

            if !saw_quit {
                if let Err(e) = db.save() {
                    eprintln!("Auto-save on exit: {}", e);
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

    loop {
        if let Some(path) = &current_file {
            print!("pardusdb [{}]> ", path.display());
        } else {
            print!("pardusdb [memory]> ");
        }
            let _ = io::stdout().flush();

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


