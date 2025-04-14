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
            let affects = db.delete_todo_by_branch_order_number(&branch, index)?;
            if affects > 0 {
                println!("DONE! Good Job!")
            } else {
                println!("Nothing is DONE!")
            };
        }
        Command::UI(branch) => {
            ui::run(branch, db);
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
    UI(String),
    Help,
}

impl Command {
    fn parse_from_args() -> Result<Command, error::Error> {
        let branch = git::get_current_branch()?;
        let args: Vec<String> = std::env::args().collect();
        if args.len() == 1 {
            return Ok(Command::List(branch, false));
        }
        let arg_1 = &args[1];
        if args.len() == 2 {
            if ["-a".to_string(), "--all".to_string(), "--all-branches".to_string()].contains(arg_1) {
                return Ok(Command::List(branch, true));
            }
            if ["--ui".to_string(), "-i".to_string(), "ui".to_string(), "i".to_string()].contains(arg_1) {
                return Ok(Command::UI(branch));
            }
            if ["-h".to_string(), "--help".to_string()].contains(arg_1) {
                return Ok(Command::Help);
            }
            if ["done".to_string(), "-".to_string()].contains(arg_1) {
                return Err(error::Error::from("missing done index"));
            }
        }
        if args.len() == 3 && ["done".to_string(), "-".to_string()].contains(arg_1) {
            let branch_index: Vec<String> = args[2].split(':').map(String::from).collect();
            let (branch, index) = if branch_index.len() == 1 { (branch, args[2].clone()) } else { (branch_index[0].clone(), branch_index[1].clone()) };
            let index = match index.parse::<i32>() {
                Ok(index) => index,
                Err(err) => return Err(error::Error::from_normal_error(err)),
            };
            return Ok(Command::Done(branch, index));
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

mod ui {
    use crate::dao;
    use crate::dao::Todo;
    use color::palette::css::LIGHT_GRAY;
    use floem::event::EventListener;
    use floem::kurbo::Size;
    use floem::reactive::create_effect;
    use floem::window::{WindowButtons, WindowConfig};
    use floem::{prelude::*, IntoView};
    use im::Vector;
    use std::rc::Rc;

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

        VirtualStack::list_with_view(
            move || todo_list.get().enumerate(),
            move |(index, (state, item))| {
                let db0 = db.clone();

                let checkbox_state = RwSignal::new(state);
                create_effect(move |_| {
                    if checkbox_state.get() {
                        todo_list.update(|list| {
                            db0.delete_todo(item.id).expect("failed to delete item");
                            list.remove(index);
                        });
                    }
                });

                (checkmark(checkbox_state), label(item.content)).h_stack().style(move |s| {
                    s.flex_row()
                        .width_full()
                        .items_center()
                        .height(item_height)
                        .apply_if(index != 0, |s| s.border_top(1.0).border_color(LIGHT_GRAY))
                })
            },
        )
        .style(move |s| s.flex_col().flex_grow(1.0))
        .scroll()
        .style(move |s| s.width_full().height_full().border(1.0))
        .on_event_stop(EventListener::WindowClosed, move |_| std::process::exit(0))
        .on_event_stop(EventListener::DoubleClick, move |_| std::process::exit(0))
    }
}
