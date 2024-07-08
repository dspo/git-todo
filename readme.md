# git todo

`git todo` list/add/done todo item on current branch.

## Usage

![20240629015816_rec_](https://github.com/dspo/git-todo/assets/25881576/806cb7d9-25ff-4ea7-9cb6-a464e25a9318)

### Add a todo item on current

`git todo <your todo item description>`

```shell
git todo implment api
# Added it!
```

```shell
git todo add test cases
# Added it!
```

### List todo items on current branch

`git todo`

```shell
git todo
# 1  implement api
# 2  add test cases
```

### List todo items on all branches

`git todo -a` or `git todo --all` or `git todo --all-branches`

```shell
git todo -a

# *feat/some-feature
#        1  implement api
#        2  add test cases
# main
#        1  fix some issue
```

### Mark some todo item as done on the current branch

`git todo done <index>` or `git todo - <index>`

```shell
git todo
# 1  implement api
# 2  add test cases

git todo done 1
# DONE! Good Job!

git todo
# 1 add test cases

git todo - 1
# DONE! Good Job!

git todo # response empty
```

### Mark some todo item as done on the specific branch

`git todo done <branch>:<index>` or `git todo - <branch>:<index>

```shell
git todo -a
# *feat/some-feature
#        1  implement api
#        2  add test cases
# main
#        1  fix some issue

git todo done main:1 
# DONE! Good Job!

git todo -a
# *feat/some-feature
#        1  implement api
#        2  add test cases
```

## install

### brew

```shell
brew install dspo/tools/git-todo
```

### build

```shell
cargo build -r
```
