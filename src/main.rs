fn main() {
    if let Err(err) = execute() {
        println!("{}", err)
    }
}

fn execute() -> Result<(), error::Error> {
    let db = dao::DatabaseAccess::create_database_access(".git/info/todo.sqlite")?;
    db.create_table_if_not_exists()?;

    let command = Command::parse_from_args()?;
    match command {
        Command::List(branch, true) => {
            let items = db.list_all_todos()?;
            let items = items.iter().enumerate();
            let mut last_branch = "";
            let mut index = 0;
            for (_, item) in items {
                if item.branch.as_str().ne(last_branch) {
                    index = 0;
                    last_branch = &item.branch;
                    if item.branch.eq(&branch) {
                        println!("*{}", item.branch);
                    } else {
                        println!(" {}", item.branch);
                    }
                }
                index += 1;
                println!("\t{}  {}", index, item.content);
            }
        }
        Command::List(branch, false) => {
            let items = db.list_todos_on_branch(&branch)?;
            let items = items.iter().enumerate();
            for (index, item) in items {
                println!("{}  {}", index + 1, item.content);
            }
        }
        Command::Todo(branch, content) => {
            let affects = db.create_todo(&branch, &content)?;
            if affects > 0 {
                println!("Added it!")
            } else {
                println!("Nothing is added!")
            };
        }
        Command::Done(branch, index) => {
            let affects = db.delete_todo(&branch, index)?;
            if affects > 0 {
                println!("DONE! Good Job!")
            } else {
                println!("Nothing is DONE!")
            };
        }
        Command::Help => {
            println!("More usages see https://github.com/dspo/git-todo?tab=readme-ov-file#usage");
        }
    };
    Ok(())
}

enum Command {
    List(String, bool),
    Todo(String, String),
    Done(String, i32),
    Help,
}

impl Command {
    fn parse_from_args() -> Result<Command, error::Error> {
        let branch = git::get_current_branch()?;
        let args: Vec<String> = std::env::args().collect();
        if args.len() <= 1 {
            return Ok(Command::List(branch, false));
        }
        if args.len() <= 2 && ([String::from("-a"), String::from("--all"), String::from("--all-branches")].contains(&args[1])) {
            return Ok(Command::List(branch, true));
        }
        if args[1] == "done" || args[1] == "-" {
            if args.len() < 3 {
                return Err(error::Error::from("missing done index"));
            }
            let branch_index: Vec<String> = args[2].split(':').map(String::from).collect();
            let (branch, index) = if branch_index.len() == 1 { (branch, args[2].clone()) } else { (branch_index[0].clone(), branch_index[1].clone()) };
            let index = match index.parse::<i32>() {
                Ok(index) => index,
                Err(err) => return Err(error::Error::from_normal_error(err)),
            };
            return Ok(Command::Done(branch, index));
        }
        if args[1] == "-h" || args[1] == "--help" {
            return Ok(Command::Help);
        }
        Ok(Command::Todo(branch, args[1..].join(" ")))
    }
}

mod dao {
    use rusqlite::Connection;

    pub(crate) struct DatabaseAccess(Connection);

    impl DatabaseAccess {
        pub(crate) fn create_database_access(path: &str) -> Result<DatabaseAccess, rusqlite::Error> {
            Ok(DatabaseAccess(Connection::open(path)?))
        }

        pub(crate) fn create_table_if_not_exists(&self) -> Result<usize, rusqlite::Error> {
            self.0.execute(
                "CREATE TABLE IF NOT EXISTS todos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            branch TEXT NOT NULL,
            content TEXT NOT NULL
        )",
                (),
            )
        }

        pub(crate) fn create_todo(&self, branch: &str, content: &str) -> Result<usize, rusqlite::Error> {
            self.0.execute("INSERT INTO todos (branch, content) VALUES (?1, ?2)", (branch, content))
        }

        pub(crate) fn list_todos_on_branch(&self, branch: &str) -> Result<Vec<Todo>, rusqlite::Error> {
            let mut stmt = self.0.prepare("SELECT id, branch, content FROM todos WHERE branch = ?1 ORDER BY id ASC")?;
            let todos = stmt.query_map([branch], |row| {
                Ok(Todo {
                    id: row.get(0)?,
                    branch: row.get(1)?,
                    content: row.get(2)?,
                })
            })?;
            let todos = todos.into_iter().flatten().collect();
            Ok(todos)
        }

        pub(crate) fn list_all_todos(&self) -> Result<Vec<Todo>, rusqlite::Error> {
            let mut stmt = self.0.prepare("SELECT id, branch, content FROM todos ORDER BY branch, id ASC")?;
            let todos = stmt.query_map([], |row| {
                Ok(Todo {
                    id: row.get(0)?,
                    branch: row.get(1)?,
                    content: row.get(2)?,
                })
            })?;
            let mut list: Vec<Todo> = Vec::new();
            for item in todos.flatten() {
                list.push(item);
            }
            Ok(list)
        }

        pub(crate) fn delete_todo(&self, branch: &str, order_number: i32) -> Result<usize, rusqlite::Error> {
            let items = self.list_todos_on_branch(branch)?;
            for (index, item) in items.iter().enumerate() {
                if index + 1 == order_number as usize {
                    return self.0.execute("DELETE FROM todos WHERE id = ?1", (item.id,));
                }
            }
            Ok(0)
        }
    }

    #[derive(Debug)]
    pub struct Todo {
        id: i32,
        pub(crate) branch: String,
        pub(crate) content: String,
    }
}

mod git {
    use crate::error;

    pub fn get_current_branch() -> Result<String, error::Error> {
        let output = std::process::Command::new("git").arg("symbolic-ref").arg("--short").arg("HEAD").output()?;

        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(branch);
        }
        Err(error::Error::Other(format!("failed to execute 'git symbolic-ref --short HEAD': {}", output.status)))
    }
}

mod error {
    #[derive(Debug)]
    pub enum Error {
        IO(std::io::Error),
        SQLite(rusqlite::Error),
        Other(String),
    }

    impl Error {
        pub fn from_normal_error<E: std::fmt::Display>(err: E) -> Self {
            Self::Other(format!("{}", err))
        }
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::IO(ref err) => write!(f, "{err}"),
                Error::SQLite(ref err) => write!(f, "{err}"),
                Error::Other(msg) => write!(f, "{msg}"),
            }
        }
    }

    impl From<std::io::Error> for Error {
        fn from(err: std::io::Error) -> Self {
            Error::IO(err)
        }
    }

    impl From<rusqlite::Error> for Error {
        fn from(err: rusqlite::Error) -> Self {
            Error::SQLite(err)
        }
    }

    impl From<String> for Error {
        fn from(msg: String) -> Self {
            Error::Other(msg)
        }
    }

    impl From<&str> for Error {
        fn from(value: &str) -> Self {
            Self::from(value.to_string())
        }
    }
}
