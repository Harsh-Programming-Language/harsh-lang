# 12. An I/O project: a small grep

Everything so far, in one program. `grep` searches a file for lines containing a string; this chapter builds a small one, `minigrep`, the way a real program gets built: a first version that works, then a refactor into something you would not be embarrassed by, then tests, then a feature. Along the way: command-line arguments, reading a file, error output on the right stream, an exit code, an environment variable, and the split between a library that does the work and a binary that talks to the shell. Each stage is a complete project; the output under it is the program run with the arguments shown.

## 12.1 Reading arguments

`poem.txt`

```text
I'm nobody! Who are you?
Are you nobody, too?
Then there's a pair of us - don't tell!
They'd banish us, you know.

How dreary to be somebody!
How public, like a frog
To tell your name the livelong day
To an admiring bog!
```

`src/main.hrs`

```
use std.env

fn main$:
    let args: Vec<String> = env.args$ <- collect$
    let query = &args[1]
    let file_path = &args[2]

    println! "Searching for {query}"
    println! "In file {file_path}"
```

```text
$ cargo run -- searchstring example-filename.txt
Searching for searchstring
In file example-filename.txt
```

`env.args$` is an iterator over the arguments, and `<- collect$` gathers them into a `Vec<String>` — the type annotation tells `collect` which collection to build, as in chapter 1. Element `0` is the program's own name, so the query is `args[1]` and the file is `args[2]`; `&args[1]` borrows the string out of the vector rather than moving it. (The `$ cargo run -- …` line above the output shows the invocation: the `--` separates cargo's arguments from the program's.)

## 12.2 Reading the file

`src/main.hrs`

```
use std.env
use std.fs

fn main$:
    let args: Vec<String> = env.args$ <- collect$
    let query = &args[1]
    let file_path = &args[2]

    println! "Searching for {query}"
    println! "In file {file_path}"

    let contents =
        fs.read_to_string file_path
            <- expect "Should have been able to read the file"

    println! "With text:\n{contents}"
```

```text
$ cargo run -- the poem.txt
Searching for the
In file poem.txt
With text:
I'm nobody! Who are you?
Are you nobody, too?
Then there's a pair of us - don't tell!
They'd banish us, you know.

How dreary to be somebody!
How public, like a frog
To tell your name the livelong day
To an admiring bog!
```

`fs.read_to_string file_path` reads the whole file into a `String`, or fails, and for now the failure is an `expect`. The program works. It is also a `main` doing three jobs — parsing, reading, printing — with two unlabelled positional arguments and an error strategy of "crash". Programs written like this are how big `main` functions happen, so the next section takes it apart before it grows.

## 12.3 Refactoring

Two things at once: give the configuration a type, and give errors a path that is not a panic. First the type:

`src/main.hrs`

```
use std.env
use std.fs

struct Config
    query: String
    file_path: String

impl Config
    fn new args: &[String] -> Config:
        if args <- len$ < 3:
            panic! "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        Config\ query, file_path

fn main$:
    let args: Vec<String> = env.args$ <- collect$
    let config = Config.new (&args)
    println! "Searching for {}" (config <- query)
    println! "In file {}" (config <- file_path)

    let contents =
        fs.read_to_string (config <- file_path)
            <- expect "Should have been able to read the file"

    println! "With text:\n{contents}"
```

```text
$ cargo run
thread 'main' panicked at src/main.hrs:12:
not enough arguments
```

`Config` names the two values, and `Config.new` builds one from the argument list — cloning the strings, which costs a little and keeps the ownership story simple: the `Config` owns its fields, and `args` is untouched. The check for too few arguments is in the one place the arguments are looked at. Run with none and the check fires — as a panic, which is the right *check* with the wrong *response*: a panic is for bugs, and a user forgetting an argument is not a bug.

`src/main.hrs`

```
use std.env
use std.fs
use std.process

struct Config
    query: String
    file_path: String

impl Config
    fn build args: &[String] -> Result<Config, &'static str>:
        if args <- len$ < 3:
            return Err "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        Ok (Config\ query, file_path)

fn main$:
    let args: Vec<String> = env.args$ <- collect$

    let config = Config.build (&args) <- unwrap_or_else |err|:
        println! "Problem parsing arguments: {err}"
        process.exit 1

    println! "Searching for {}" (config <- query)
    println! "In file {}" (config <- file_path)

    let contents =
        fs.read_to_string (config <- file_path)
            <- expect "Should have been able to read the file"

    println! "With text:\n{contents}"
```

```text
$ cargo run
Problem parsing arguments: not enough arguments
```

Now `Config.build` returns `Result<Config, &'static str>` — an `Err` with a message, in the case the caller should handle — and `main` handles it with `unwrap_or_else`: given the `Ok`, the value; given the `Err`, run this closure with it. The closure prints the message and calls `process.exit 1`, which ends the program with a nonzero code and *no* panic output. Compare the two reports: the second is what a user should see.

The closure is a *trailing closure with a block body*: `<- unwrap_or_else |err|:` opens a block whose two lines are the closure's body, and the argument list closes after them. It is the form you will write for every callback of more than one line, and it is chapter 13's subject.

### Into a library

The last refactoring moves the logic out of `main.hrs` into `lib.hrs`, so that it can be tested — a binary's `main` cannot be called from a test, but a library's functions can. Here is the library, tests first, in the shape the finished program will have:

```
use std.env
use std.error.Error
use std.fs

pub struct Config
    pub query: String
    pub file_path: String
    pub ignore_case: bool

impl Config
    pub fn build args: &[String] -> Result<Config, &'static str>:
        if args <- len$ < 3:
            return Err "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        let ignore_case = env.var "IGNORE_CASE" <- is_ok$
        Ok (Config\ query, file_path, ignore_case)

pub fn run config: Config -> Result<(), Box<dyn Error>>:
    let contents = fs.read_to_string (config <- file_path)?

    let results =
        if config <- ignore_case:
            search_case_insensitive (&config <- query) (&contents)

        else:
            search (&config <- query) (&contents)

    for line in results:
        println! "{line}"

    Ok ()

pub fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- contains query:
            results <- push line

    results

pub fn search_case_insensitive<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let query = query <- to_lowercase$
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- to_lowercase$ <- contains (&query):
            results <- push line

    results

#[cfg test]
mod tests
    use super.*

    #[test]
    fn case_sensitive$:
        let query = "duct"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Duct tape."
        assert_eq!
            (vec! ["safe, fast, productive."])
            (search query contents)

    #[test]
    fn case_insensitive$:
        let query = "rUsT"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Trust me."
        assert_eq!
            (vec! ["Rust:", "Trust me."])
            (search_case_insensitive query contents)
```

```text
$ cargo test
running 2 tests
test tests::case_insensitive ... ok
test tests::case_sensitive ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Two tests, two functions. `search` walks `contents <- lines$` and keeps each line that `<- contains query`; `search_case_insensitive` lowercases both first. The signature `fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>` is chapter 10's lesson in use: the result holds slices *of `contents`*, so the output lifetime is tied to that parameter and not to `query`, and the compiler holds the caller to it. `run` reads the file, chooses a search, and prints; it returns `Result<(), Box<dyn Error>>` so that `?` on the read works and any error reaches `main` as a value. The test strings use `"\` at the end of a line — Rust's line continuation inside a string, which drops the newline and the leading spaces of the next line — so the expected text starts on the line below.

The tests were written before the search functions worked; that order — a failing test, then the code that passes it — is test-driven development, and `cargo test` makes it cheap.

## 12.4 The finished program

`src/main.hrs`

```
use std.env
use std.process

use minigrep.Config

fn main$:
    let args: Vec<String> = env.args$ <- collect$

    let config = Config.build (&args) <- unwrap_or_else |err|:
        eprintln! "Problem parsing arguments: {err}"
        process.exit 1

    if let Err e = minigrep.run config:
        eprintln! "Application error: {e}"
        process.exit 1
```

`src/lib.hrs`

```
use std.env
use std.error.Error
use std.fs

pub struct Config
    pub query: String
    pub file_path: String
    pub ignore_case: bool

impl Config
    pub fn build args: &[String] -> Result<Config, &'static str>:
        if args <- len$ < 3:
            return Err "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        let ignore_case = env.var "IGNORE_CASE" <- is_ok$
        Ok (Config\ query, file_path, ignore_case)

pub fn run config: Config -> Result<(), Box<dyn Error>>:
    let contents = fs.read_to_string (config <- file_path)?

    let results =
        if config <- ignore_case:
            search_case_insensitive (&config <- query) (&contents)

        else:
            search (&config <- query) (&contents)

    for line in results:
        println! "{line}"

    Ok ()

pub fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- contains query:
            results <- push line

    results

pub fn search_case_insensitive<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let query = query <- to_lowercase$
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- to_lowercase$ <- contains (&query):
            results <- push line

    results

#[cfg test]
mod tests
    use super.*

    #[test]
    fn case_sensitive$:
        let query = "duct"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Duct tape."
        assert_eq!
            (vec! ["safe, fast, productive."])
            (search query contents)

    #[test]
    fn case_insensitive$:
        let query = "rUsT"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Trust me."
        assert_eq!
            (vec! ["Rust:", "Trust me."])
            (search_case_insensitive query contents)
```

```text
$ cargo run -- frog poem.txt
How public, like a frog
```

`main.hrs` is now short. `use minigrep.Config` imports from the library crate, which has the package's name; `Config.build` and `minigrep.run` do the work, and `main` handles the two ways they can fail. The output is one line, the one containing `frog`. A different query:

`src/main.hrs`

```
use std.env
use std.process

use minigrep.Config

fn main$:
    let args: Vec<String> = env.args$ <- collect$

    let config = Config.build (&args) <- unwrap_or_else |err|:
        eprintln! "Problem parsing arguments: {err}"
        process.exit 1

    if let Err e = minigrep.run config:
        eprintln! "Application error: {e}"
        process.exit 1
```

`src/lib.hrs`

```
use std.env
use std.error.Error
use std.fs

pub struct Config
    pub query: String
    pub file_path: String
    pub ignore_case: bool

impl Config
    pub fn build args: &[String] -> Result<Config, &'static str>:
        if args <- len$ < 3:
            return Err "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        let ignore_case = env.var "IGNORE_CASE" <- is_ok$
        Ok (Config\ query, file_path, ignore_case)

pub fn run config: Config -> Result<(), Box<dyn Error>>:
    let contents = fs.read_to_string (config <- file_path)?

    let results =
        if config <- ignore_case:
            search_case_insensitive (&config <- query) (&contents)

        else:
            search (&config <- query) (&contents)

    for line in results:
        println! "{line}"

    Ok ()

pub fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- contains query:
            results <- push line

    results

pub fn search_case_insensitive<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let query = query <- to_lowercase$
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- to_lowercase$ <- contains (&query):
            results <- push line

    results

#[cfg test]
mod tests
    use super.*

    #[test]
    fn case_sensitive$:
        let query = "duct"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Duct tape."
        assert_eq!
            (vec! ["safe, fast, productive."])
            (search query contents)

    #[test]
    fn case_insensitive$:
        let query = "rUsT"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Trust me."
        assert_eq!
            (vec! ["Rust:", "Trust me."])
            (search_case_insensitive query contents)
```

```text
$ cargo run -- body poem.txt
I'm nobody! Who are you?
Are you nobody, too?
How dreary to be somebody!
```

`body` matches `nobody` and `somebody` and not `Body` — the search is case-sensitive, and the way to change that is an environment variable, read by `Config.build`:

`src/main.hrs`

```
use std.env
use std.process

use minigrep.Config

fn main$:
    let args: Vec<String> = env.args$ <- collect$

    let config = Config.build (&args) <- unwrap_or_else |err|:
        eprintln! "Problem parsing arguments: {err}"
        process.exit 1

    if let Err e = minigrep.run config:
        eprintln! "Application error: {e}"
        process.exit 1
```

`src/lib.hrs`

```
use std.env
use std.error.Error
use std.fs

pub struct Config
    pub query: String
    pub file_path: String
    pub ignore_case: bool

impl Config
    pub fn build args: &[String] -> Result<Config, &'static str>:
        if args <- len$ < 3:
            return Err "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        let ignore_case = env.var "IGNORE_CASE" <- is_ok$
        Ok (Config\ query, file_path, ignore_case)

pub fn run config: Config -> Result<(), Box<dyn Error>>:
    let contents = fs.read_to_string (config <- file_path)?

    let results =
        if config <- ignore_case:
            search_case_insensitive (&config <- query) (&contents)

        else:
            search (&config <- query) (&contents)

    for line in results:
        println! "{line}"

    Ok ()

pub fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- contains query:
            results <- push line

    results

pub fn search_case_insensitive<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let query = query <- to_lowercase$
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- to_lowercase$ <- contains (&query):
            results <- push line

    results

#[cfg test]
mod tests
    use super.*

    #[test]
    fn case_sensitive$:
        let query = "duct"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Duct tape."
        assert_eq!
            (vec! ["safe, fast, productive."])
            (search query contents)

    #[test]
    fn case_insensitive$:
        let query = "rUsT"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Trust me."
        assert_eq!
            (vec! ["Rust:", "Trust me."])
            (search_case_insensitive query contents)
```

```text
IGNORE_CASE=1 $ cargo run -- to poem.txt
Are you nobody, too?
How dreary to be somebody!
To tell your name the livelong day
To an admiring bog!
```

`env.var "IGNORE_CASE"` is `Ok` if the variable is set, to anything, and `<- is_ok$` turns that into the `bool`. Set it and every line with `to` or `To` comes back. An environment variable is the conventional place for an option a user sets once for a session; a command-line flag would be the place for one that changes per run, and either is a small change to `build`.

### Errors go to standard error

`src/main.hrs`

```
use std.env
use std.process

use minigrep.Config

fn main$:
    let args: Vec<String> = env.args$ <- collect$

    let config = Config.build (&args) <- unwrap_or_else |err|:
        eprintln! "Problem parsing arguments: {err}"
        process.exit 1

    if let Err e = minigrep.run config:
        eprintln! "Application error: {e}"
        process.exit 1
```

`src/lib.hrs`

```
use std.env
use std.error.Error
use std.fs

pub struct Config
    pub query: String
    pub file_path: String
    pub ignore_case: bool

impl Config
    pub fn build args: &[String] -> Result<Config, &'static str>:
        if args <- len$ < 3:
            return Err "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        let ignore_case = env.var "IGNORE_CASE" <- is_ok$
        Ok (Config\ query, file_path, ignore_case)

pub fn run config: Config -> Result<(), Box<dyn Error>>:
    let contents = fs.read_to_string (config <- file_path)?

    let results =
        if config <- ignore_case:
            search_case_insensitive (&config <- query) (&contents)

        else:
            search (&config <- query) (&contents)

    for line in results:
        println! "{line}"

    Ok ()

pub fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- contains query:
            results <- push line

    results

pub fn search_case_insensitive<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let query = query <- to_lowercase$
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- to_lowercase$ <- contains (&query):
            results <- push line

    results

#[cfg test]
mod tests
    use super.*

    #[test]
    fn case_sensitive$:
        let query = "duct"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Duct tape."
        assert_eq!
            (vec! ["safe, fast, productive."])
            (search query contents)

    #[test]
    fn case_insensitive$:
        let query = "rUsT"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Trust me."
        assert_eq!
            (vec! ["Rust:", "Trust me."])
            (search_case_insensitive query contents)
```

```text
$ cargo run
Problem parsing arguments: not enough arguments
```

Run with no arguments the program prints its message and exits with code 1, and the message went to *standard error* — `eprintln!` instead of `println!` — so that `cargo run > output.txt` would leave the error on the terminal and the file empty, rather than writing the complaint into the results. Output is for results and standard error is for everything else; a program that gets this right composes with every other tool on the system, and one that gets it wrong quietly does not.

## 12.5 What you have

A whole program: `env.args$ <- collect$` for arguments, `fs.read_to_string` for a file, a `Config` type built by a `Result`-returning function, `unwrap_or_else` with a block closure and `process.exit` for a clean failure, `Box<dyn Error>` and `?` in `run`, a `lib.hrs` with the logic and tests and a `main.hrs` that only talks to the shell, `env.var` for an option, and `eprintln!` for errors. That is the shape of most command-line programs, and you have written one.

Next: closures and iterators — the two features the functional languages lend Rust, which this program used twice without explaining, and which turn most `for` loops into something shorter.
