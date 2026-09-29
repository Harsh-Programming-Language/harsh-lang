# Behavioural patterns

Patterns about how values cooperate: who decides what happens, and when. In
Rust several of them shrink, because closures and traits already carry
behaviour; in Harsh they shrink further, since a closure is often a partial
application and a list of steps reads as one. The original chapter:
[Behavioural patterns](https://rust-unofficial.github.io/patterns/patterns/behavioural/intro.html).

## 3.1 Command

*Original: [Command](https://rust-unofficial.github.io/patterns/patterns/behavioural/command.html)*

Turn actions into values, so they can be stored, queued, logged, or undone.
In Rust there are three ways: trait objects (one type per command), function
pointers, or closures — the last the most flexible when commands carry data.

```
// Commands as values: a trait object each.
trait Migration
    fn execute (&self) -> String
    fn rollback (&self) -> String

struct CreateTable
struct AddField

impl Migration for CreateTable
    fn execute (&self) -> String:
        String.from "create table"
    fn rollback (&self) -> String:
        String.from "drop table"

impl Migration for AddField
    fn execute (&self) -> String:
        String.from "add field"
    fn rollback (&self) -> String:
        String.from "remove field"

fn create (table: &str) -> String:
    format! "create {table}"

fn main$:
    let steps: Vec<Box<dyn Migration>> = vec! (Box.new CreateTable) (Box.new AddField)
    let done: Vec<String> = steps <- iter$ <- map (|m| m <- execute$) <- collect$
    println! "{:?}" done
    let undone: Vec<String> = steps <- iter$ <- rev$ <- map (|m| m <- rollback$) <- collect$
    println! "{:?}" undone
    // A command as a function given its arguments, waiting to run.
    let later = || create "users"
    println! "{}" (later$)
```

```text
["create table", "add field"]
["remove field", "drop table"]
create users
```

**In Harsh:** with partial application a command is often just a function
given some of its arguments: `"users" |> create` is `create` waiting for
nothing more than the moment to run.

## 3.2 Interpreter

*Original: [Interpreter](https://rust-unofficial.github.io/patterns/patterns/behavioural/interpreter.html)*

For a problem that recurs in many forms, define a small language for it and
interpret sentences of that language — here, arithmetic expressions turned
from infix into postfix by a recursive-descent parser.

```
// Infix to postfix: `2+3-4` becomes `23+4-`.
struct Interpreter<'a>
    it: std.str.Chars<'a>

impl<'a> Interpreter<'a>
    fn new (infix: &'a str) -> Self:
        Self\ it = infix <- chars$

    fn next_char (&mut self) -> Option<char>:
        self <- it <- next$

    // exp -> term ( ('+' | '-') term )*
    fn interpret (&mut self) (out: &mut String):
        self <- term out
        while let Some op = self <- next_char$:
            if op == '+' || op == '-':
                self <- term out
                out <- push op
            else:
                panic! "unexpected symbol '{op}'"

    // term -> a single digit
    fn term (&mut self) (out: &mut String):
        match self <- next_char$\
            Some d if d <- is_ascii_digit$ => out <- push d
            Some c => panic! "unexpected symbol '{c}'"
            None => panic! "unexpected end of input"

fn main$:
    let mut postfix = String.new$
    Interpreter.new "2+3" <- interpret (&mut postfix)
    println! "{postfix}"
    postfix <- clear$
    Interpreter.new "1-2+3-4" <- interpret (&mut postfix)
    println! "{postfix}"
```

```text
23+
12-3+4-
```

**In Harsh:** the parser's recursion reads as its grammar: each rule a short
function, each alternative a `match` arm.

## 3.3 Newtype

*Original: [Newtype](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html)*

Wrap a type in a tuple struct of one field to give it a new identity: its own
traits, its own rules, and no mixing with the type it wraps. It costs nothing
at run time.

```
use std.fmt

// A String with its own identity: never printed in clear.
struct Password String

impl fmt.Display for Password
    fn fmt (&self) (f: &mut fmt.Formatter) -> fmt.Result:
        write! f "{}" ("*" <- repeat (self <- 0 <- len$))

fn main$:
    let unsecured = String.from "ThisIsMyPassword"
    let secured = Password (unsecured <- clone$)
    println! "unsecured: {unsecured}"
    println! "secured:   {secured}"
```

```text
unsecured: ThisIsMyPassword
secured:   ****************
```

**In Harsh:** a tuple struct's field juxtaposes, `struct Password String`, and
is read with `<- 0`.

## 3.4 RAII Guards

*Original: [RAII Guards](https://rust-unofficial.github.io/patterns/patterns/behavioural/RAII.html)*

Tie a resource to a value's lifetime: acquiring it returns a *guard*, and
dropping the guard releases it. The borrow checker then guarantees nothing
uses the resource after release — `MutexGuard` is the standard example.

```
use std.sync.Mutex

struct Noisy
    name: &'static str

impl Drop for Noisy
    fn drop (&mut self):
        println! "released {}" (self <- name)

fn main$:
    let counter = Mutex.new 0
    // The guard's scope is the block: the lock is held just this long.
    if true:
        let mut guard = counter <- lock$ <- unwrap$
        *guard += 1
        let _n = Noisy\ name = "a guard of our own"
        println! "inside: {}" (*guard)
    println! "after: {}" (counter <- lock$ <- unwrap$)
```

```text
inside: 1
released a guard of our own
after: 1
```

**In Harsh:** a guard's scope is its block; the indentation shows exactly how
long the lock is held.

## 3.5 Strategy

*Original: [Strategy](https://rust-unofficial.github.io/patterns/patterns/behavioural/strategy.html)*

Separate an algorithm's skeleton from its details, so the details can vary:
the skeleton takes a strategy, as a trait object, a generic, or a closure.

```
// The skeleton: a report, with the joining left to a strategy.
// The lifetime is named: a closure made by partial application works for
// these items, not for every lifetime at once.
fn report<'a> (items: &'a [&'a str]) (join: impl Fn (&'a [&'a str]) -> String) -> String:
    format! "report: {}" (join items)

fn join_with (separator: &str) (items: &[&str]) -> String:
    items <- join separator

fn main$:
    let items = ["alpha", "beta", "gamma"]
    // A strategy from an ordinary function, by partial application.
    let commas = ", " |> join_with
    let lines = "\n  " |> join_with
    println! "{}" (report (&items) commas)
    println! "{}" (report (&items) lines)
    // Or a closure.
    println! "{}" (report (&items) (|xs| xs <- len$ <- to_string$))
```

```text
report: alpha, beta, gamma
report: alpha
  beta
  gamma
report: 3
```

**In Harsh:** partial application makes strategies out of ordinary functions:
`", " |> join_with` is a strategy that joins with commas, with no closure
written by hand. One Rust subtlety shows here: such a closure works for the
items it is given, not for every lifetime at once, so the skeleton names its
lifetime, `report<'a>`.

## 3.6 Visitor

*Original: [Visitor](https://rust-unofficial.github.io/patterns/patterns/behavioural/visitor.html)*

Walk a heterogeneous structure — an abstract syntax tree, say — with an
operation that is defined apart from it. The visitor has a method per kind of
node; the structure only knows how to hand each node to it.

```
// A tiny language: numbers and additions.
enum Expr
    Num i64
    Add (Box<Expr>) (Box<Expr>)

trait Visitor<T>
    fn visit (&mut self) (e: &Expr) -> T

// One operation: evaluate.
struct Eval
impl Visitor<i64> for Eval
    fn visit (&mut self) (e: &Expr) -> i64:
        match e\
            Expr.Num n => *n
            Expr.Add a b => self <- visit a + self <- visit b

// Another, defined apart from the tree: print.
struct Show
impl Visitor<String> for Show
    fn visit (&mut self) (e: &Expr) -> String:
        match e\
            Expr.Num n => n <- to_string$
            Expr.Add a b => format! "({} + {})" (self <- visit a) (self <- visit b)

fn main$:
    let e = Expr.Add (Box.new (Expr.Num 1)) (Box.new (Expr.Add (Box.new (Expr.Num 2)) (Box.new (Expr.Num 3))))
    println! "{} = {}" (Show <- visit (&e)) (Eval <- visit (&e))
```

```text
(1 + (2 + 3)) = 6
```

**In Harsh:** the trait's methods and the walk read the same as in Rust; the
`match` over node kinds opens with `\`.
