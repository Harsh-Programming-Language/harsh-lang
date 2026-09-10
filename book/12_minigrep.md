# 12. An I/O project: a small grep

Everything so far, in one program. `grep` searches a file for lines containing a string; this chapter builds a small one, `minigrep`, the way a real program gets built: a first version that works, then a refactor into something you would not be embarrassed by, then tests, then a feature. Along the way: command-line arguments, reading a file, error output on the right stream, an exit code, an environment variable, and the split between a library that does the work and a binary that talks to the shell. Each stage is a complete project; the output under it is the program run with the arguments shown.

## 12.1 Reading arguments

@@ args searchstring example-filename.txt

`env.args$` is an iterator over the arguments, and `<- collect$` gathers them into a `Vec<String>` — the type annotation tells `collect` which collection to build, as in chapter 1. Element `0` is the program's own name, so the query is `args[1]` and the file is `args[2]`; `&args[1]` borrows the string out of the vector rather than moving it. (The `$ cargo run -- …` line above the output shows the invocation: the `--` separates cargo's arguments from the program's.)

## 12.2 Reading the file

@@ read_file the poem.txt

`fs.read_to_string file_path` reads the whole file into a `String`, or fails, and for now the failure is an `expect`. The program works. It is also a `main` doing three jobs — parsing, reading, printing — with two unlabelled positional arguments and an error strategy of "crash". Programs written like this are how big `main` functions happen, so the next section takes it apart before it grows.

## 12.3 Refactoring

Two things at once: give the configuration a type, and give errors a path that is not a panic. First the type:

@@ config_panic !panic

`Config` names the two values, and `Config.new` builds one from the argument list — cloning the strings, which costs a little and keeps the ownership story simple: the `Config` owns its fields, and `args` is untouched. The check for too few arguments is in the one place the arguments are looked at. Run with none and the check fires — as a panic, which is the right *check* with the wrong *response*: a panic is for bugs, and a user forgetting an argument is not a bug.

@@ config_result !panic

Now `Config.build` returns `Result<Config, &'static str>` — an `Err` with a message, in the case the caller should handle — and `main` handles it with `unwrap_or_else`: given the `Ok`, the value; given the `Err`, run this closure with it. The closure prints the message and calls `process.exit 1`, which ends the program with a nonzero code and *no* panic output. Compare the two reports: the second is what a user should see.

The closure is a *trailing closure with a block body*: `<- unwrap_or_else |err|:` opens a block whose two lines are the closure's body, and the argument list closes after them. It is the form you will write for every callback of more than one line, and it is chapter 13's subject.

### Into a library

The last refactoring moves the logic out of `main.hrs` into `lib.hrs`, so that it can be tested — a binary's `main` cannot be called from a test, but a library's functions can. Here is the library, tests first, in the shape the finished program will have:

@@ search_tests !test

Two tests, two functions. `search` walks `contents <- lines$` and keeps each line that `<- contains query`; `search_case_insensitive` lowercases both first. The signature `fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>` is chapter 10's lesson in use: the result holds slices *of `contents`*, so the output lifetime is tied to that parameter and not to `query`, and the compiler holds the caller to it. `run` reads the file, chooses a search, and prints; it returns `Result<(), Box<dyn Error>>` so that `?` on the read works and any error reaches `main` as a value. The test strings use `"\` at the end of a line — Rust's line continuation inside a string, which drops the newline and the leading spaces of the next line — so the expected text starts on the line below.

The tests were written before the search functions worked; that order — a failing test, then the code that passes it — is test-driven development, and `cargo test` makes it cheap.

## 12.4 The finished program

@@ minigrep frog poem.txt

`main.hrs` is now short. `use minigrep.Config` imports from the library crate, which has the package's name; `Config.build` and `minigrep.run` do the work, and `main` handles the two ways they can fail. The output is one line, the one containing `frog`. A different query:

@@ minigrep body poem.txt

`body` matches `nobody` and `somebody` and not `Body` — the search is case-sensitive, and the way to change that is an environment variable, read by `Config.build`:

@@ minigrep to poem.txt IGNORE_CASE=1

`env.var "IGNORE_CASE"` is `Ok` if the variable is set, to anything, and `<- is_ok$` turns that into the `bool`. Set it and every line with `to` or `To` comes back. An environment variable is the conventional place for an option a user sets once for a session; a command-line flag would be the place for one that changes per run, and either is a small change to `build`.

### Errors go to standard error

@@ minigrep !panic

Run with no arguments the program prints its message and exits with code 1, and the message went to *standard error* — `eprintln!` instead of `println!` — so that `cargo run > output.txt` would leave the error on the terminal and the file empty, rather than writing the complaint into the results. Output is for results and standard error is for everything else; a program that gets this right composes with every other tool on the system, and one that gets it wrong quietly does not.

## 12.5 What you have

A whole program: `env.args$ <- collect$` for arguments, `fs.read_to_string` for a file, a `Config` type built by a `Result`-returning function, `unwrap_or_else` with a block closure and `process.exit` for a clean failure, `Box<dyn Error>` and `?` in `run`, a `lib.hrs` with the logic and tests and a `main.hrs` that only talks to the shell, `env.var` for an option, and `eprintln!` for errors. That is the shape of most command-line programs, and you have written one.

Next: closures and iterators — the two features the functional languages lend Rust, which this program used twice without explaining, and which turn most `for` loops into something shorter.
