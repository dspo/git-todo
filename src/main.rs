use rusqlite::{Connection};

fn main() {
    if let Err(err) = execute() { println!("{}", err) }
}

fn execute() -> Result<(), String> {
    let branch = match git::get_current_branch() {
        Ok(branch) => branch,
        Err(err) => return Err(format!("failed to get current branch: {}", err))
    };

    let db = Database::create_connection(".git/info/todo.sqlite")?;
    let _ = db.create_table_if_not_exists();

    let command = Command::parse_from_args()?;
    match command {
        Command::List => {
            let items = db.list_todos_on_branch(&branch)?;
            let items = items.iter().enumerate();
            for (index, item) in items {
                println!("{}\t{}", index + 1, item.content);
            }
        }
        Command::Todo(content) => {
            let affects = db.create_todo(&branch, &content);
            if affects > 0 { println!("Added it!") } else { println!("Nothing is added!") };
        }
        Command::Done(index) => {
            let affects = db.delete_todo(&branch, index)?;
            if affects > 0 { println!("DONE! Good Job!") } else { println!("Nothing is DONE!") };
        }
        Command::Help => {
            println!("Usage: todo [command] [args]");
            println!("Commands:");
            println!("  git todo               - List all todos");
            println!("  git todo [text [text]] - Add a new todo");
            println!("  git todo done [index|-]  - Mark a todo as done");
        }
    };
    Ok(())
}

enum Command {
    List,
    Todo(String),
    Done(i32),
    Help,
}

impl Command {
    fn parse_from_args() -> Result<Command, String> {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 2 {
            return Ok(Command::List);
        }
        if args[1] == "done" || args[1] == "-" {
            if args.len() < 3 {
                return Err("missing done index".to_string());
            }
            let index = match args[2].parse::<i32>() {
                Ok(index) => index,
                Err(err) => return Err(format!("{}", err)),
            };
            return Ok(Command::Done(index));
        }
        if args[1] == "-h" || args[1] == "--help" {
            return Ok(Command::Help);
        }
        Ok(Command::Todo(args[1..].join(" ")))
    }
}

struct Database {
    conn: Connection,
}

impl Database {
    fn create_connection(path: &str) -> Result<Database, String> {
        match Connection::open(path) {
            Ok(conn) => Ok(Database { conn }),
            Err(err) => Err(format!("failed to write: {}", err)),
        }
    }

    fn create_table_if_not_exists(&self) -> usize {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS todos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            branch TEXT NOT NULL,
            content TEXT NOT NULL
        )",
            (),
        ).unwrap_or_default()
    }

    fn create_todo(&self, branch: &str, content: &str) -> usize {
        self.conn.execute(
            "INSERT INTO todos (branch, content) VALUES (?1, ?2)",
            (branch, content),
        ).unwrap_or_default()
    }

    fn list_todos_on_branch(&self, branch: &str) -> Result<Vec<Todo>, String> {
        let mut stmt = self.conn.prepare("SELECT id, branch, content FROM todos WHERE branch = ?1 ORDER BY id ASC").unwrap();
        let todos = stmt
            .query_map([branch, ], |row| {
                Ok(Todo { id: row.get(0)?, branch: row.get(1)?, content: row.get(2)? })
            });
        let todos = match todos {
            Ok(todos) => todos,
            Err(err) => return Err(format!("failed to list items: {}", err))
        };
        let todos = todos.into_iter().map(|todo| todo.unwrap()).collect();
        Ok(todos)
    }

    fn delete_todo(&self, branch: &str, order_number: i32) -> Result<usize, String> {
        let items = self.list_todos_on_branch(branch)?;
        let items = items.iter().enumerate();
        for (index, item) in items {
            if index + 1 == order_number as usize {
                return Ok(self.conn.execute("DELETE FROM todos WHERE id = ?1", (item.id,)).unwrap_or_default());
            }
        }
        Ok(0)
    }
}

#[derive(Debug)]
struct Todo {
    id: i32,
    #[allow(dead_code)]
    branch: String,
    content: String,
}

mod git {
    pub fn get_current_branch() -> Result<String, String> {
        let output = std::process::Command::new("git")
            .arg("symbolic-ref")
            .arg("--short")
            .arg("HEAD")
            .output()
            .expect("failed to execute 'git symbolic-ref --short HEAD'");

        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(branch);
        }
        Err(format!("failed to execute 'git symbolic-ref --short HEAD': {}", output.status))
    }
}
