# git todo

`git todo` list/add/done todo item on current branch.

```shell
$ git todo -h
Usage: todo [command] [args]
Commands:
  git todo               - List all todos
  git todo [text [text]] - Add a new todo
  git todo done [index]  - Mark a todo as done. Alias '-'
```

![20240629015816_rec_](https://github.com/dspo/git-todo/assets/25881576/806cb7d9-25ff-4ea7-9cb6-a464e25a9318)

## install

### brew

```shell
brew install dspo/tools/git-todo
```

### build

```shell
cargo build -r
```
