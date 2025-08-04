use clap::{error::ErrorKind, Parser};

#[cfg(feature = "ui")]
use std::process::Command;

/// A Git extension for managing todo items on branches
#[derive(Parser, Debug)]
#[command(author, version, about = "A Git extension for managing todo items on branches", long_about = None)]
#[command(propagate_version = true)]
enum GitTodo {
    /// List todo items
    ///
    /// Without any flags, lists todos on the current branch.
    /// Use -a or --all to list todos from all branches.
    List {
        /// List todos from all branches
        #[arg(short = 'a', long = "all", long = "all-branches")]
        all: bool,
    },

    /// Add a new todo item
    ///
    /// Add a new todo item to the current branch.
    Todo {
        /// The todo item description
        description: String,
    },

    /// Mark a todo item as done
    ///
    /// Mark a todo item as done by its index.
    /// By default, operates on the current branch.
    /// Use -b or --branch to specify a different branch.
    Done {
        /// The index of the todo item to mark as done
        index: u32,

        /// The branch where the todo item is located
        #[arg(short = 'b', long = "branch")]
        branch: Option<String>,
    },

    /// Open the UI interface
    ///
    /// Launch the interactive UI for managing todos.
    UI,
}

fn main() {
    #[cfg(feature = "ui")]
    ui_mode();

    if let Err(err) = execute() {
        println!("{}", err)
    }
}

#[cfg(feature = "ui")]
fn ui_mode() {
    // Check if running in UI mode
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() >= 2 && args[1] == "--ui-mode" {
        let branch = if args.len() >= 3 { &args[2] } else { "main" };
        match dao::DatabaseAccess::create_database_access(".git/info/todo.sqlite") {
            Ok(db) => {
                ui::run(branch.to_string(), db);
            }
            Err(e) => {
                println!("Failed to create database connection for UI: {}", e);
            }
        }
    }
}

fn execute() -> Result<(), error::Error> {
    let db = dao::DatabaseAccess::create_database_access(".git/info/todo.sqlite")?;
    db.create_table_if_not_exists()?;

    let current_branch = git::get_current_branch()?;
    let args = std::env::args().collect::<Vec<String>>();

    // Handle no arguments case (default to list)
    let git_todo = if args.len() == 1 {
        GitTodo::List { all: false }
    } else if args.len() == 3 && args[1] == "-" {
        // Handle git-todo - <index> format (equivalent to git-todo done <index>)
        match args[2].parse::<u32>() {
            Ok(index) => GitTodo::Done { index, branch: None },
            Err(_) => {
                // Treat as normal todo if index is not a number
                let description = args[1..].join(" ");
                GitTodo::Todo { description }
            }
        }
    } else {
        // Try to parse normally
        match GitTodo::try_parse() {
            Ok(parsed) => parsed,
            Err(e) => {
                // For unknown commands, treat as todo item
                if e.kind() == ErrorKind::UnknownArgument || e.kind() == ErrorKind::DisplayHelp || e.kind() == ErrorKind::DisplayVersion {
                    // Print help for help/version requests or invalid arguments
                    e.print()?;
                    return Ok(());
                }
                // Treat all other arguments as todo description
                let description = args[1..].join(" ");
                GitTodo::Todo { description }
            }
        }
    };

    match git_todo {
        GitTodo::List { all } => {
            if all {
                let items = db.list_all_todos()?;
                let items = items.iter().enumerate();
                let mut last_branch = "";
                let mut index = 0;
                for (_, item) in items {
                    if item.branch.as_str().ne(last_branch) {
                        index = 0;
                        last_branch = &item.branch;
                        if item.branch.eq(&current_branch) {
                            println!("*{}", item.branch);
                        } else {
                            println!(" {}", item.branch);
                        }
                    }
                    index += 1;
                    println!("\t{}  {}", index, item.content);
                }
            } else {
                let items = db.list_todos_on_branch(&current_branch)?;
                let items = items.iter().enumerate();
                for (index, item) in items {
                    println!("{}  {}", index + 1, item.content);
                }
            }
        }
        GitTodo::Todo { description } => {
            let affects = db.create_todo(&current_branch, &description)?;
            if affects > 0 {
                println!("Added it!")
            } else {
                println!("Nothing is added!")
            };
        }
        GitTodo::Done { index, branch } => {
            let target_branch = branch.unwrap_or(current_branch);
            let affects = db.delete_todo_by_branch_order_number(&target_branch, index as i32)?;
            if affects > 0 {
                println!("DONE! Good Job!")
            } else {
                println!("Nothing is DONE!")
            };
        }
        GitTodo::UI => {
            #[cfg(feature = "ui")]
            {
                // Run UI in a separate process to ensure it continues running after command line exits
                let exe_path = std::env::current_exe().expect("Failed to get current executable path");
                match Command::new(exe_path)
                    .arg("--ui-mode")
                    .arg(&current_branch)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    Err(e) => {
                        println!("Failed to start UI: {}", e);
                    }
                }
            }
            #[cfg(not(feature = "ui"))]
            println!("UI feature is not enabled. Please build with '--features ui' to use this command.");
        }
    };
    Ok(())
}

// The original Command enum and its implementation are removed as they're no longer needed

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

        pub(crate) fn delete_todo_by_branch_order_number(&self, branch: &str, order_number: i32) -> Result<usize, rusqlite::Error> {
            let items = self.list_todos_on_branch(branch)?;
            for (index, item) in items.iter().enumerate() {
                if index + 1 == order_number as usize {
                    return self.delete_todo(item.id);
                }
            }
            Ok(0)
        }

        pub(crate) fn delete_todo(&self, id: i32) -> Result<usize, rusqlite::Error> {
            self.0.execute("DELETE FROM todos WHERE id = ?1", (id,))
        }
    }

    #[derive(Debug, Clone, Hash)]
    pub struct Todo {
        pub(crate) id: i32,
        pub(crate) branch: String,
        pub(crate) content: String,
    }

    impl PartialEq for Todo {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }

    impl Eq for Todo {}
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

#[cfg(feature = "ui")]
mod ui {
    use crate::dao;
    use crate::dao::Todo;
    use color::palette::css::LIGHT_GRAY;
    use floem::event::EventListener;
    use floem::kurbo::Size;
    use floem::reactive::create_effect;
    use floem::text::Weight;
    use floem::window::{WindowButtons, WindowConfig};
    use floem::{prelude::*, IntoView};
    use im::Vector;
    use std::rc::Rc;

    // Source: https://www.svgrepo.com/svg/505349/cross | License: MIT
    pub const CROSS_SVG: &str = r##"
<svg width="800px" height="800px" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M19 5L5 19M5.00001 5L19 19" stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"##;

    pub(crate) fn run(branch: String, db: dao::DatabaseAccess) {
        let title = &branch.clone();
        let app = floem::Application::new().window(
            move |_| enhanced_list(&branch, db),
            Some(
                WindowConfig::default()
                    .size(Size::new(300.0, 500.0))
                    .resizable(true)
                    .title(title)
                    .with_mac_os_config(|c| c.hide_titlebar(false).hide_titlebar_buttons(true).enable_shadow(false).transparent_title_bar(true))
                    .enabled_buttons(WindowButtons::CLOSE),
            ),
        );
        app.run()
    }

    pub(crate) fn enhanced_list(branch: &str, db: dao::DatabaseAccess) -> impl IntoView {
        let db = Rc::new(db);
        let branch = branch.to_string();
        let todos = db.list_todos_on_branch(&branch).expect("");
        let todo_list: Vector<(bool, Todo)> = todos.into_iter().map(|item| (false, item)).collect();
        let todo_list = RwSignal::new(todo_list);

        let item_height = 24.0;

        let checkmark = |checkbox_state| Checkbox::new_rw(checkbox_state).style(|s| s.margin_left(6));

        let label = |item: String| item.style(|s| s.margin_left(6).height(32.0).font_size(22.0).items_center());

        let x_mark = {
            let db = db.clone();
            move |index, item_id: i32| {
                svg(CROSS_SVG)
                    .on_click_stop({
                        let db = db.clone();
                        move |_| {
                            todo_list.update(|list| {
                                db.delete_todo(item_id).expect("failed to delete item");
                                list.remove(index);
                            });
                        }
                    })
                    .style(|s| {
                        s.size(18.0, 18.)
                            .font_weight(Weight::BOLD)
                            .color(palette::css::RED)
                            .border(1.0)
                            .border_color(palette::css::RED)
                            .border_radius(16.0)
                            .padding(2.)
                            .margin_right(20.0)
                            .hover(|s| s.color(palette::css::WHITE).background(palette::css::RED))
                    })
            }
        };

        VirtualStack::list_with_view(move || todo_list.get().enumerate(), {
            let db = db.clone();
            move |(index, (state, item)): (usize, (bool, Todo))| {
                let checkbox_state = RwSignal::new(state);
                create_effect({
                    let db = db.clone();
                    move |_| {
                        if checkbox_state.get() {
                            todo_list.update(|list| {
                                db.delete_todo(item.id).expect("failed to delete item");
                                list.remove(index);
                            });
                        }
                    }
                });

                (checkmark(checkbox_state), label(item.content), x_mark(index, item.id)).h_stack().style(move |s| {
                    s.flex_row()
                        .width_full()
                        .items_center()
                        .height(item_height)
                        .apply_if(index != 0, |s| s.border_top(1.0).border_color(LIGHT_GRAY))
                })
            }
        })
        .style(move |s| s.flex_col().flex_grow(1.0))
        .scroll()
        .style(move |s| s.width_full().height_full().border(1.0))
        .on_event_stop(EventListener::WindowClosed, move |_| std::process::exit(0))
        .on_event_stop(EventListener::DoubleClick, move |_| std::process::exit(0))
    }
}
