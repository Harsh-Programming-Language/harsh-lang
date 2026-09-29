# Harsh Design Patterns

*A Harsh companion to [Rust Design Patterns](https://rust-unofficial.github.io/patterns/) — its idioms, patterns and
anti-patterns, in the same order, with every program written in Harsh and
built and run to produce the output you see.*

A design pattern is a solution that has worked before, to a problem that keeps
coming back: a way to arrange code that other programmers will recognise. Rust
has its own, because it is not an object-oriented language: ownership, the
borrow checker, traits and closures make some classic patterns unnecessary and
call for others. *Rust Design Patterns*, a community book, collects them in
three families — **idioms**, the habits of the community; **design patterns**,
arrangements that solve a recurring problem; and **anti-patterns**, arrangements
that look like solutions and create problems.

Harsh is Rust with another layout, so every one of them holds in Harsh as it
is. What this companion adds is how each one is written in Harsh, and where
Harsh changes the picture: pipes and partial application make several patterns
a line long, comprehensions replace some loops outright, and the layout rules
make a builder or a command list read top to bottom. Where Harsh changes
nothing, the page says so and keeps short.

## How to read it

Each page gives the idea in a few sentences, a program in Harsh with its real
output, what Harsh changes, and a link to the original page — the place for
the full discussion, the trade-offs and the history, which this companion does
not repeat. If you have not read *The Harsh Programming Language* yet, read it
first: this book assumes you can read Harsh.

The book follows the original's order: the idioms; the design patterns —
behavioural, creational, structural, and for foreign functions; the
anti-patterns; functional programming; and, to close, the design principles
behind them all.

*Rust Design Patterns* is by its contributors, under the Mozilla Public
License 2.0, as is this companion; see ATTRIBUTION.md.


# Idioms

Idioms are the community's habits: small, local choices that make Rust code
read as Rust. They are not rules — break one when there is a reason — but code
that follows them is easier for everyone else to read. The original chapter:
[Idioms](https://rust-unofficial.github.io/patterns/idioms/index.html).

## 2.1 Use borrowed types for arguments

*Original: [Use borrowed types for arguments](https://rust-unofficial.github.io/patterns/idioms/coercion-arguments.html)*

Take the most general borrowed form a function can work with: `&str` rather
than `&String`, `&[T]` rather than `&Vec<T>`, `&T` rather than `&Box<T>`. The
caller's `&String` coerces to `&str` on its own, so the function accepts more
— a string literal, a slice of another string — at no cost.

```
// `&str`, not `&String`: every string the caller has will do.
fn three_vowels (word: &str) -> bool:
    let mut count = 0
    for c in word <- chars$:
        match c\
            'a' | 'e' | 'i' | 'o' | 'u' => count += 1
            _ => count = 0
        if count >= 3:
            return true
    false

fn main$:
    let owned = String.from "Ferris"
    let sentence = "Once upon a time, there was a friendly curious crab named Ferris"
    println! "{}: {}" owned (three_vowels (&owned))
    for word in sentence <- split ' ':
        if three_vowels word:
            println! "{word} has three consecutive vowels!"
```

```text
Ferris: false
curious has three consecutive vowels!
```

**In Harsh:** nothing changes; the coercion is Rust's. An argument that is not a single atom takes
parentheses: `three_vowels (&owned)`.

## 2.2 Concatenating Strings with format!

*Original: [Concatenating Strings with format!](https://rust-unofficial.github.io/patterns/idioms/concat-format.html)*

To build a string from pieces, `format!` is usually clearer than a series of
`push_str` calls: the shape of the result is visible at once. For a string
built in a loop, or where every allocation counts, pushing into one `String`
remains the faster choice.

```
fn greet (name: &str) -> String:
    // The shape of the result, at a glance.
    format! "Hello {name}!"

fn main$:
    println! "{}" (greet "Harsh")
    // Built piece by piece, where the pieces come from a loop.
    let mut line = String.new$
    for part in ["a", "b", "c"]:
        line <- push_str part
    println! "{line}"
```

```text
Hello Harsh!
abc
```

**In Harsh:** `format!` takes its arguments by juxtaposition, and names can
sit inside the braces, so most calls need no arguments after the string.

## 2.3 Constructor

*Original: [Constructor](https://rust-unofficial.github.io/patterns/idioms/ctor.html)*

Rust has no constructors in the language: by convention, an associated
function `new` builds a value. When a type has an obvious default, implement
`Default` too, and let `new` call it or be it.

```
#[derive Debug]
pub struct Second
    value: u64

impl Second
    // By convention, `new` builds the value.
    pub fn new (value: u64) -> Self:
        Self\ value = value

    pub fn value (&self) -> u64:
        self <- value

impl Default for Second
    fn default$ -> Self:
        Self\ value = 0

fn main$:
    let s = Second.new 42
    println! "{}" (s <- value$)
    println! "{:?}" (Second.default$)
```

```text
42
Second { value: 0 }
```

**In Harsh:** `Second.new 5` calls the associated function — the path uses a
dot, and the argument follows the name.

## 2.4 The Default Trait

*Original: [The Default Trait](https://rust-unofficial.github.io/patterns/idioms/default.html)*

`Default` gives a type a value "when nothing else was said". Derived, it fills
every field with its own default; with struct-update syntax, a caller names
only the fields that differ. Containers and generic code rely on it
(`unwrap_or_default`, `mem::take`).

```
#[derive Debug Default]
struct Config
    name: String
    verbose: bool
    retries: u8
    tags: Vec<String>

fn main$:
    // Every field its own default.
    let plain = Config.default$
    println! "{:?}" plain
    // Only what differs, the rest from the default.
    let loud = Config\ verbose = true, retries = 3, ..Config.default$
    println! "{:?}" loud
    // Containers use it too.
    let missing: Option<Vec<i32>> = None
    println! "{:?}" (missing <- unwrap_or_default$)
```

```text
Config { name: "", verbose: false, retries: 0, tags: [] }
Config { name: "", verbose: true, retries: 3, tags: [] }
[]
```

**In Harsh:** struct update is written inside the literal, `Config\ verbose =
true, ..Config.default$` — `$` calls a function that takes nothing.

## 2.5 Collections Are Smart Pointers

*Original: [Collections Are Smart Pointers](https://rust-unofficial.github.io/patterns/idioms/deref.html)*

A collection that owns its data should implement `Deref` to the borrowed view
of it — `Vec<T>` to `[T]`, `String` to `str`. Every method of the borrowed view
is then available on the owner, and a function written for the view accepts
the owner too.

```
// Written for the borrowed view: a slice.
fn total (xs: &[i32]) -> i32:
    xs <- iter$ <- sum$

fn main$:
    let scores = vec! 3 1 4 1 5
    // A `&Vec<i32>` is accepted where `&[i32]` is asked for.
    println! "{}" (total (&scores))
    // The slice's methods, reached through the Vec.
    println! "{:?} {}" (scores <- first$) (scores <- contains (&4))
```

```text
14
Some(3) true
```

**In Harsh:** the same. Method calls reach through `Deref` exactly as in Rust:
`scores <- first$` is the slice's `first`, found through the `Vec`.

## 2.6 Finalisation in Destructors

*Original: [Finalisation in Destructors](https://rust-unofficial.github.io/patterns/idioms/dtor-finally.html)*

Rust has no `finally`. Code that must run however a function ends — returning
early, with `?`, or panicking — goes into the `Drop` of a value created at the
start: when the function ends, the value goes out of scope, and its destructor
runs.

```
struct Cleanup
    what: &'static str

impl Drop for Cleanup
    fn drop (&mut self):
        println! "cleaning up after {}" (self <- what)

fn work (fail: bool) -> Result<(), String>:
    let _guard = Cleanup\ what = "work"
    if fail:
        // Returning early: the guard still runs.
        return Err (String.from "failed")
    println! "working"
    Ok ()

fn main$:
    println! "{:?}" (work false)
    println! "{:?}" (work true)
```

```text
working
cleaning up after work
Ok(())
cleaning up after work
Err("failed")
```

**In Harsh:** `impl Drop for Cleanup` with `fn drop (&mut self):` beneath —
a declaration takes no `:`, its method does.

## 2.7 mem::{take(_), replace(_)}

*Original: [mem::{take(_), replace(_)}](https://rust-unofficial.github.io/patterns/idioms/mem-replace.html)*

To change an enum from one variant to another while keeping an owned field —
without cloning it — take the field out with `std.mem.take` (which leaves the
type's default in its place) or `std.mem.replace` (which leaves what you give
it), then build the new variant from it.

```
#[derive Debug]
enum State
    Draft
        text: String
    Published
        text: String
        views: u32

// From one variant to the other, keeping the text: moved out, not cloned.
fn publish (s: &mut State):
    if let State.Draft\ text = s:
        let text = std.mem.take text
        *s = State.Published\ text = text, views = 0

fn main$:
    let mut post = State.Draft\ text = String.from "Hello"
    publish (&mut post)
    println! "{:?}" post
```

```text
Published { text: "Hello", views: 0 }
```

**In Harsh:** the `match` opens with `\` and its arms bind fields by name,
`State.Draft\ text => …`, the same mark as the literal.

## 2.8 On-Stack Dynamic Dispatch

*Original: [On-Stack Dynamic Dispatch](https://rust-unofficial.github.io/patterns/idioms/on-stack-dyn-dispatch.html)*

To choose at run time between values of different types behind one trait,
there is no need to `Box` them: declare a variable for each type, initialise
only the one the condition picks — *deferred conditional initialisation* —
and take a `&dyn Trait` to it. The value lives on the stack; only the reference
is dynamic.

```
use std.fmt.Display

fn main$:
    for big in [true, false]:
        // Declared, not initialised: only the branch taken gives one a value.
        let number: i32
        let text: String
        let shown: &dyn Display = if big:
            number = 42
            &number
        else:
            text = String.from "forty-two"
            &text
        println! "{shown}"
```

```text
42
forty-two
```

**In Harsh:** each branch of the `if` initialises its own variable and ends
with the reference to it; the variables are declared, without a value, before
the `if`.

## 2.9 Foreign function interface (FFI)

*Original: [Foreign function interface (FFI)](https://rust-unofficial.github.io/patterns/idioms/ffi/intro.html)*

Three idioms for code on either side of a C boundary.

### 2.9.1 Idiomatic Errors

*Original: [Idiomatic Errors](https://rust-unofficial.github.io/patterns/idioms/ffi/errors.html)*

C has no `Result`: errors cross the boundary as integers. A **flat enum**
becomes its `#[repr(C)]` discriminant; a **structured enum**, whose variants
carry data, is mapped to one code per variant; a **custom error type** can give
C a code and, separately, a message as a C string the caller frees. Never let
a panic cross the boundary.

```
// A flat enum: its discriminant is the code.
#[repr C]
#[derive Clone Copy Debug]
pub enum DatabaseError
    IsReadOnly = 1
    IoError = 2
    FileCorrupted = 3

// A structured enum: one code per variant, the data left behind.
pub enum ParseError
    Empty
    BadChar char
    TooLong usize

impl ParseError
    pub fn code (&self) -> i32:
        match self\
            ParseError.Empty => 1
            ParseError.BadChar _ => 2
            ParseError.TooLong _ => 3

// For C, only the codes cross.
pub extern "C" fn parse_status (input: i32) -> i32:
    let err = match input\
        0 => ParseError.Empty
        1 => ParseError.BadChar '?'
        _ => ParseError.TooLong 99
    err <- code$

fn main$:
    println! "{}" (DatabaseError.FileCorrupted as i32)
    println! "{} {} {}" (parse_status 0) (parse_status 1) (parse_status 2)
```

```text
3
1 2 3
```

**In Harsh:** `#[repr C]` takes its argument by juxtaposition; the mapping from
a structured enum to codes is a `match` opening with `\`.

### 2.9.2 Accepting Strings

*Original: [Accepting Strings](https://rust-unofficial.github.io/patterns/idioms/ffi/accepting-strings.html)*

A string that comes from C stays C's: borrow it as `&CStr` with
`CStr::from_ptr`, for no longer than the call, and keep the `unsafe` to that one
line — check the pointer, borrow, then work in safe code.

```
use std.ffi.CStr
use std.os.raw.c_char

// A string from C, borrowed for the call: the unsafe is one line.
pub extern "C" fn log_message (text: *const c_char) -> i32:
    if text <- is_null$:
        return -1
    let message = unsafe: CStr.from_ptr text
    // From here on, safe code: a lossy conversion copes with bad UTF-8.
    println! "log: {}" (message <- to_string_lossy$)
    0

fn main$:
    // As C would call it.
    let owned = std.ffi.CString.new "hello from C" <- unwrap$
    println! "{}" (log_message (owned <- as_ptr$))
    println! "{}" (log_message (std.ptr.null$))
```

```text
log: hello from C
0
-1
```

**In Harsh:** the one unsafe line is `unsafe: CStr.from_ptr text`; everything
after it is ordinary Harsh.

### 2.9.3 Passing Strings

*Original: [Passing Strings](https://rust-unofficial.github.io/patterns/idioms/ffi/passing-strings.html)*

To hand a string to C: make the owned `CString` live as long as C may use the
pointer — bind it, never pass `CString::new(..).as_ptr()` from a temporary;
keep the unsafe code to the call; if C writes into the buffer, pass a
`Vec<u8>` instead; and, unless the API says otherwise, keep ownership on the
Rust side.

```
use std.ffi.CString
use std.os.raw.c_char

extern "C"
    fn strlen (s: *const c_char) -> usize

fn main$:
    // Bound: it lives to the end of the block, as long as C may read it.
    let owned = CString.new "passed to C" <- unwrap$
    let n = unsafe: strlen (owned <- as_ptr$)
    println! "C counted {n} bytes"
```

```text
C counted 11 bytes
```

**In Harsh:** the `CString` is bound by a `let`, so its lifetime is the
block's, visible in the indentation.

## 2.10 Iterating over an Option

*Original: [Iterating over an Option](https://rust-unofficial.github.io/patterns/idioms/option-iter.html)*

`Option` is a collection of zero or one element: it implements
`IntoIterator`. It can extend a collection, be chained onto an iterator, or be
looped over — no `if let` needed.

```
fn main$:
    let mut names = vec! "Ada" "Grace"
    let extra: Option<&str> = Some "Barbara"
    // An Option extends a collection like any iterator.
    names <- extend extra
    println! "{:?}" names
    // It chains onto one.
    let all: Vec<&str> = names <- iter$ <- copied$ <- chain (Some "Linus") <- collect$
    println! "{:?}" all
    // And it loops: zero times or once.
    for name in extra:
        println! "hello {name}"
```

```text
["Ada", "Grace", "Barbara"]
["Ada", "Grace", "Barbara", "Linus"]
hello Barbara
```

**In Harsh:** `for name in extra:` loops over an `Option` as over anything;
chains read top to bottom with `<-`.

## 2.11 Pass Variables to Closure

*Original: [Pass Variables to Closure](https://rust-unofficial.github.io/patterns/idioms/pass-var-to-closure.html)*

A `move` closure takes ownership of everything it uses. To decide variable by
variable — clone this one, borrow that one — prepare them in a block just
before the closure, and let the closure be the block's value.

```
use std.rc.Rc

fn main$:
    let num1 = Rc.new 1
    let num2 = Rc.new 2
    let num3 = Rc.new 3
    // Prepared next to the closure: num2 cloned, num3 borrowed.
    let closure = do:
        let num2 = num2 <- clone$
        let num3 = num3 <- as_ref$
        move ||: *num1 + *num2 + *num3
    println! "{}" (closure$)
    // num2 is still ours.
    println! "{num2}"
```

```text
6
2
```

**In Harsh:** the block is `do:` with its lines beneath, and its last line is
its value: the preparation and the closure read as one unit.

## 2.12 Privacy For Extensibility

*Original: [Privacy For Extensibility](https://rust-unofficial.github.io/patterns/idioms/priv-extend.html)*

A public struct with all-public fields can never gain a field without
breaking everyone who builds it with a literal. Mark it `#[non_exhaustive]`
(or give it one private field) and provide a constructor: fields can then be
added in a later version.

```
mod shapes
    // Fields may be added later without breaking anyone: no literal outside.
    #[non_exhaustive]
    #[derive Debug]
    pub struct Circle
        pub radius: f64

    impl Circle
        pub fn new (radius: f64) -> Self:
            Self\ radius = radius

fn main$:
    let c = shapes.Circle.new 2.0
    println! "{:?} r = {}" c (c <- radius)
```

```text
Circle { radius: 2.0 } r = 2
```

**In Harsh:** `mod shapes` holds its items beneath, as every declaration does;
the attribute is `#[non_exhaustive]` above the struct.

## 2.13 Easy doc initialization

*Original: [Easy doc initialization](https://rust-unofficial.github.io/patterns/idioms/rustdoc-init.html)*

When every documentation example would need the same setup, wrap the example
in a function that takes the prepared value as a parameter: the documentation
test compiles the function but never calls it, so the setup is never written,
and the example shows only what it documents. A helper that builds the value
is the alternative when the example must run.

```
pub struct Connection
    pub name: String

impl Connection
    /// Sends a message.
    ///
    /// The example is a function taking the connection: compiled, never
    /// called, so no connection has to be set up to show `send`.
    ///
    /// ```
    /// fn call_send (c: Connection):
    ///     c <- send "hi"
    /// ```
    pub fn send (&self) (msg: &str):
        println! "{} <- {msg}" (self <- name)

fn main$:
    let c = Connection\ name = String.from "test"
    c <- send "hi"
```

```text
test <- hi
```

**In Harsh:** documentation examples in `///` comments are written in Harsh,
and `hrs test` transpiles and runs them — with the same fences as Rust's.

## 2.14 Temporary mutability

*Original: [Temporary mutability](https://rust-unofficial.github.io/patterns/idioms/temporary-mutability.html)*

When data must be prepared mutably and then only read, shadow it: build it
in a nested block (or a mutable binding) and rebind it immutable. The
compiler then refuses any later change.

```
fn main$:
    // Prepared mutably in a block, then bound immutable.
    let data = do:
        let mut d = vec! 5 3 9 1
        d <- sort$
        d
    println! "{:?}" data
    // Or shadowed: the same name, no longer `mut`.
    let mut total = 0
    for x in &data:
        total += x
    let total = total
    println! "{total}"
```

```text
[1, 3, 5, 9]
18
```

**In Harsh:** the nested block is `do:`; its value is its last line, and the
`let` that receives it is immutable.

## 2.15 Return consumed arg on error

*Original: [Return consumed arg on error](https://rust-unofficial.github.io/patterns/idioms/return-consumed-arg-on-error.html)*

A function that takes ownership of an argument and may fail should hand the
argument back in its error, so the caller can try again without having cloned
it first — as `String::from_utf8` returns the bytes in its `FromUtf8Error`.

```
// The argument, handed back in the error.
#[derive Debug]
pub struct SendError String

pub fn send (value: String) (attempt: u32) -> Result<(), SendError>:
    if attempt < 2:
        return Err (SendError value)
    println! "sent {value:?} on attempt {attempt}"
    Ok ()

fn main$:
    let mut value = String.from "important"
    for attempt in 0..3:
        match send value attempt\
            Ok () => break
            Err (SendError v) =>
                println! "attempt {attempt} failed; retrying"
                value = v
```

```text
attempt 0 failed; retrying
attempt 1 failed; retrying
sent "important" on attempt 2
```

**In Harsh:** the error is a tuple struct, `SendError String`, taken apart by
juxtaposition: `Err (SendError v) => …`.


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


# Creational patterns

Patterns for building values. The original chapter:
[Creational patterns](https://rust-unofficial.github.io/patterns/patterns/creational/intro.html).

## 4.1 Builder

*Original: [Builder](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html)*

When a value has many optional parts, build it step by step through a
*builder*: each method sets one part and returns the builder, and `build`
makes the value. Rust has no named or default arguments; this is its answer.

```
#[derive Debug]
pub struct Request
    url: String
    method: String
    headers: Vec<(String, String)>
    body: Option<String>

pub struct RequestBuilder
    request: Request

impl RequestBuilder
    pub fn new (url: &str) -> Self:
        let request = Request\ url = url <- to_string$, method = String.from "GET", headers = Vec.new$, body = None
        Self\ request = request

    pub fn method (mut self) (m: &str) -> Self:
        self <- request <- method = m <- to_string$
        self

    pub fn header (mut self) (k: &str) (v: &str) -> Self:
        self <- request <- headers <- push (k <- to_string$, v <- to_string$)
        self

    pub fn body (mut self) (b: &str) -> Self:
        self <- request <- body = Some (b <- to_string$)
        self

    pub fn build (self) -> Request:
        self <- request

fn main$:
    // One line per choice: the layout of a builder is its list of options.
    let req =
        RequestBuilder.new "https://harsh-lang.com"
          <- method "POST"
          <- header "Accept" "text/html"
          <- body "hello"
          <- build$
    println! "{:#?}" req
```

```text
Request {
    url: "https://harsh-lang.com",
    method: "POST",
    headers: [
        (
            "Accept",
            "text/html",
        ),
    ],
    body: Some(
        "hello",
    ),
}
```

**In Harsh:** a chain is one line per step, `<-` at the start of each — the
layout of a builder in Harsh is the list of its choices.

## 4.2 Fold

*Original: [Fold](https://rust-unofficial.github.io/patterns/patterns/creational/fold.html)*

Run a transformation over every node of a structure, producing a new
structure: a *folder* has a method per kind of node, each returning the
rebuilt node — the default rebuilding it unchanged, so a folder overrides only
what it changes.

```
// A small tree of names and calls.
#[derive Debug]
enum Node
    Name String
    Call String (Vec<Node>)

// A folder: a method per kind of node, each returning the rebuilt node.
trait Folder
    fn fold_name (&mut self) (n: String) -> Node:
        Node.Name n
    fn fold_call (&mut self) (f: String) (args: Vec<Node>) -> Node:
        let args = args <- into_iter$ <- map (|a| self <- fold a) <- collect$
        Node.Call f args
    fn fold (&mut self) (n: Node) -> Node:
        match n\
            Node.Name s => self <- fold_name s
            Node.Call f args => self <- fold_call f args

// This folder overrides one method: it renames every name.
struct Renamer
impl Folder for Renamer
    fn fold_name (&mut self) (n: String) -> Node:
        Node.Name (format! "{n}_renamed")

fn main$:
    let tree = Node.Call (String.from "f") (vec! (Node.Name (String.from "x")) (Node.Name (String.from "y")))
    println! "{:?}" (Renamer <- fold tree)
```

```text
Call("f", [Name("x_renamed"), Name("y_renamed")])
```

**In Harsh:** the default methods sit in the trait, and a folder that renames
identifiers overrides one of them.


# Structural patterns

Patterns for arranging types and modules. The original chapter:
[Structural patterns](https://rust-unofficial.github.io/patterns/patterns/structural/intro.html).

## 5.1 Compose Structs

*Original: [Compose Structs](https://rust-unofficial.github.io/patterns/patterns/structural/compose-structs.html)*

A large struct borrowed as a whole cannot lend two of its parts at once to
different functions. Split it into smaller structs, and each function borrows
only the part it needs — the borrow checker then sees they do not overlap.

```
struct Connection
    url: String
struct Users
    names: Vec<String>

// Two parts instead of one large struct: each can be borrowed alone.
struct Database
    connection: Connection
    users: Users

fn log_connection (c: &Connection):
    println! "connected to {}" (c <- url)

fn add_user (u: &mut Users) (name: &str):
    u <- names <- push (name <- to_string$)

fn main$:
    let connection = Connection\ url = String.from "db://local"
    let users = Users\ names = Vec.new$
    let mut db = Database\ connection = connection, users = users
    // A shared borrow of one part, a mutable one of the other: no conflict.
    let c = &db <- connection
    add_user (&mut db <- users) "ada"
    log_connection c
    println! "{:?}" (db <- users <- names)
```

```text
connected to db://local
["ada"]
```

**In Harsh:** field access is `<-`, so `db <- connection` and `db <- users`
are the two independent borrows.

## 5.2 Prefer Small Crates

*Original: [Prefer Small Crates](https://rust-unofficial.github.io/patterns/patterns/structural/small-crates.html)*

Small crates that do one thing are easier to understand, reuse and compile
in parallel; the cost is more dependencies to manage. Harsh works with the
same crates as Rust — every crate on crates.io is a Harsh dependency — and a
Harsh library of your own is shared as Rust with `hrs export`, or as Harsh by
path (*The Harsh Programming Language*, chapter 17).

There is no program on this page: the pattern is about how a project is cut,
not about code.

## 5.3 Contain unsafety in small modules

*Original: [Contain unsafety in small modules](https://rust-unofficial.github.io/patterns/patterns/structural/unsafe-mods.html)*

Keep `unsafe` code in the smallest module that can hold it, with a safe
interface around it: the module's invariants are then checked in one place,
and the rest of the program cannot break them.

```
mod ascii
    // The invariant, kept in this module alone: the bytes are ASCII.
    pub struct AsciiString
        bytes: Vec<u8>

    impl AsciiString
        pub fn new (s: &str) -> Option<Self>:
            if s <- is_ascii$:
                Some (Self\ bytes = s <- as_bytes$ <- to_vec$)
            else:
                None

        pub fn as_str (&self) -> &str:
            // Safe, because `new` checked the invariant.
            unsafe: std.str.from_utf8_unchecked (&self <- bytes)

fn main$:
    let a = ascii.AsciiString.new "hello" <- unwrap$
    println! "{}" (a <- as_str$)
    println! "{}" (ascii.AsciiString.new "héllo" <- is_none$)
```

```text
hello
true
```

**In Harsh:** `unsafe:` opens a block like any keyword; the module's safe
functions around it are ordinary Harsh.

## 5.4 Avoid complex type bounds with custom traits

*Original: [Avoid complex type bounds with custom traits](https://rust-unofficial.github.io/patterns/patterns/structural/trait-for-bounds.html)*

A struct generic over closures carries their whole signatures as bounds —
`G: FnMut() -> Result<T, Error>`, and a `T` just to name the output — and every
`impl` repeats them. Name the shape once: a trait with an **associated type**
for the output, implemented for every closure of that shape. The struct then
asks for `G: Getter`, and `T` disappears from its parameters.

```
use std.fmt.Display

#[derive Debug]
pub struct Error

#[derive Debug]
pub enum Status
    Fresh
    Stale

// The closure's shape, named once: its output is an associated type.
pub trait Getter
    type Output: Display
    fn get_value (&mut self) -> Result<Self.Output, Error>

// Every closure of that shape is a Getter.
impl<F: FnMut$ -> Result<T, Error>, T: Display> Getter for F
    type Output = T
    fn get_value (&mut self) -> Result<T, Error>:
        self$

// No `T`, and no closure signature, in the struct's parameters.
pub struct Value<G: Getter, S: Fn (&G.Output) -> Status>
    value: Option<G.Output>
    getter: G
    status: S

impl<G: Getter, S: Fn (&G.Output) -> Status> Value<G, S>
    pub fn update (&mut self) -> Result<(), Error>:
        self <- value = Some ((self <- getter <- get_value$)?)
        Ok ()

    pub fn status (&self) -> Option<Status>:
        self <- value <- as_ref$ <- map (&self <- status)

fn main$:
    let mut n = 0
    let getter = move ||:
        n += 1
        let r: Result<i32, Error> = Ok n
        r
    let mut v = Value\ value = None, getter = getter, status = |x: &i32| if *x > 1: Status.Stale else: Status.Fresh
    for _ in 0..2:
        v <- update$ <- unwrap$
        println! "{:?} {:?}" (v <- value) (v <- status$)
```

```text
Some(1) Some(Fresh)
Some(2) Some(Stale)
```

A related variant, for a list of bounds rather than a closure's shape: a trait
with those bounds as supertraits, and a blanket implementation for every type
that meets them.

```
use std.fmt.( Debug, Display)

// The long list of bounds, named once.
trait Loggable: Debug + Display + Clone + PartialEq
impl<T: Debug + Display + Clone + PartialEq> Loggable for T

// Each function asks for the one trait.
fn log_twice<T: Loggable> (x: &T):
    let copy = T.clone x
    println! "{x} / {copy:?} / same: {}" (*x == copy)

fn main$:
    log_twice (&42)
    log_twice (&String.from "hello")
```

```text
42 / 42 / same: true
hello / "hello" / same: true
```

**In Harsh:** `Self.Output` and `G.Output` are the associated type's paths,
with dots. In the variant, the trait and its blanket impl are one line each,
with no body —
`trait Loggable: Debug + Display + …` and `impl<T: …> Loggable for T` —
and come out as Rust's `{}` bodies. (Writing this page found that such
empty-bodied headers were misread, a `:` in them taken for a block; fixed in
Harsh 0.1.44.)


# Foreign function interface

Patterns for code that crosses into C and back. The original chapter:
[FFI patterns](https://rust-unofficial.github.io/patterns/patterns/ffi/intro.html).

## 6.1 Object-Based APIs

*Original: [Object-Based APIs](https://rust-unofficial.github.io/patterns/patterns/ffi/export.html)*

Expose a Rust type to C as an opaque handle: C gets a pointer it cannot look
inside, and every operation is a function taking that pointer. Ownership is
explicit — one function creates the object (`Box::into_raw`), one destroys it
(`Box::from_raw`).

```
// An opaque object for C: C holds the pointer, never the inside.
pub struct Counter
    count: u64

pub extern "C" fn counter_new$ -> *mut Counter:
    Box.into_raw (Box.new (Counter\ count = 0))

pub extern "C" fn counter_add (c: *mut Counter) (n: u64):
    let c = unsafe: &mut *c
    c <- count += n

pub extern "C" fn counter_get (c: *const Counter) -> u64:
    unsafe: (*c) <- count

pub extern "C" fn counter_free (c: *mut Counter):
    if !c <- is_null$:
        drop (unsafe: Box.from_raw c)

fn main$:
    // As C would use it.
    let c = counter_new$
    counter_add c 5
    counter_add c 7
    println! "{}" (counter_get c)
    counter_free c
```

```text
12
```

**In Harsh:** the exported functions are `pub extern "C" fn`, and each
dereference of the handle sits in its own `unsafe:` block.

## 6.2 Type Consolidation into Wrappers

*Original: [Type Consolidation into Wrappers](https://rust-unofficial.github.io/patterns/patterns/ffi/wrappers.html)*

Rust types with lifetimes do not cross into C. Wrap the owner and the state
that borrows from it into one owned type — here, a collection and a cursor
over it — and expose that single type instead.

```
// For C: one owned type, where Rust would have a collection and an iterator
// borrowing from it -- a lifetime C cannot express.
pub struct Words
    words: Vec<String>
    next: usize

impl Words
    pub fn new (text: &str) -> Self:
        Self\ words = text <- split_whitespace$ <- map String.from <- collect$, next = 0

    pub fn next (&mut self) -> Option<&str>:
        let w = self <- words <- get (self <- next)?
        self <- next += 1
        Some (w <- as_str$)

fn main$:
    let mut w = Words.new "one two three"
    while let Some word = w <- next$:
        println! "{word}"
```

```text
one
two
three
```

**In Harsh:** the wrapper is a plain struct with a `next` method; the
lifetime that C could not express is gone from its interface.


# Anti-patterns

Arrangements that look like solutions and create more problems than they
solve. Knowing them is half of avoiding them. The original chapter:
[Anti-patterns](https://rust-unofficial.github.io/patterns/anti_patterns/index.html).

## 7.1 Clone to satisfy the borrow checker

*Original: [Clone to satisfy the borrow checker](https://rust-unofficial.github.io/patterns/anti_patterns/borrow_clone.html)*

When the borrow checker refuses code, cloning the value makes the error go
away — and often hides the real mistake: two copies now drift apart, and the
change made to one is missing from the other. First the refusal:

```
fn main$:
    let mut names = vec! (String.from "Ada")
    let first = &names[0]
    // Refused: `names` is borrowed by `first` while we push.
    names <- push (String.from "Grace")
    println! "{first}"
```

```text
error[E0502]: cannot borrow `names` as mutable because it is also borrowed as immutable
 --> clone_refused.hrs:5:5
  |
3 |     let first = &names[0];
  |                  ----- immutable borrow occurs here
4 |     // Refused: `names` is borrowed by `first` while we push.
5 |     names.push(String::from("Grace"));
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ mutable borrow occurs here
6 |     println!("{first}")
  |               ------- immutable borrow later used here

error: aborting due to previous error

For more information about this error, try `rustc --explain E0502`.
```

then the fix that keeps one value, by ending the first borrow before the
second begins, instead of cloning:

```
fn main$:
    let mut names = vec! (String.from "Ada")
    // The first borrow ends before the second begins: one value, no clone.
    println! "{}" (&names[0])
    names <- push (String.from "Grace")
    println! "{:?}" names
```

```text
Ada
["Ada", "Grace"]
```

**In Harsh:** rustc's error points at the `.hrs` line it concerns, so the
borrow to shorten is found where you wrote it.

## 7.2 #[deny(warnings)]

*Original: [#[deny(warnings)]](https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html)*

Denying all warnings in the source makes a build fail on the next compiler
that adds a warning — breaking code that did nothing wrong, and every crate
that depends on it. Deny specific lints you care about, or deny warnings in
continuous integration only (`RUSTFLAGS="-D warnings"`), never in the crate.

```
// Deny the specific lints you mean, not every warning a future compiler adds.
#![deny unused_must_use]

fn checked (n: i32) -> Result<i32, String>:
    if n >= 0: Ok n else: Err (String.from "negative")

fn main$:
    // With `unused_must_use` denied, ignoring this Result would not compile.
    let r = checked 5
    println! "{:?}" r
```

```text
Ok(5)
```

**In Harsh:** attributes take their arguments by juxtaposition, `#![deny
unused_must_use]`; and `hrs check` passes cargo's flags through, so CI's
`-D warnings` works as for Rust.

## 7.3 Deref Polymorphism

*Original: [Deref Polymorphism](https://rust-unofficial.github.io/patterns/anti_patterns/deref.html)*

Implementing `Deref` to make one struct "inherit" another's methods imitates
inheritance, and misleads: `Deref` is for smart pointers, the target's traits
are not inherited, and generic code does not see through it. Compose instead,
and forward the few methods that should be shared.

```
struct Animal
    name: String

impl Animal
    fn name (&self) -> &str:
        &self <- name

// Composition, not `Deref` to Animal: the shared method is forwarded.
struct Dog
    inner: Animal
    good: bool

impl Dog
    fn name (&self) -> &str:
        self <- inner <- name$
    fn describe (&self) -> String:
        format! "{} ({})" (self <- name$) (if self <- good: "good dog" else: "dog")

fn main$:
    let d = Dog\ inner = Animal\ name = String.from "Rex", good = true
    println! "{}" (d <- describe$)
```

```text
Rex (good dog)
```

**In Harsh:** the forwarding method is one line, `self <- inner <- name$`.


# Functional Programming

Rust borrows much from functional languages — closures, iterators, algebraic
types — and Harsh leans further that way: pipes, partial application and
comprehensions are its own. This chapter is where the two books differ most.
The original chapter: [Functional usage of Rust](https://rust-unofficial.github.io/patterns/functional/index.html).

## 8.1 Programming paradigms

*Original: [Programming paradigms](https://rust-unofficial.github.io/patterns/functional/paradigms.html)*

Imperative code says *how*: a counter, a loop, a mutation per step.
Declarative code says *what*: the result as a composition of operations. Rust
supports both; iterators make the declarative form idiomatic. Harsh adds two
more declarative forms — the pipe and the comprehension.

```
fn square (n: u32) -> u32:
    n * n

fn report (total: u32) -> String:
    format! "the sum of squares is {total}"

fn main$:
    // Imperative: how.
    let mut total = 0
    for n in 1..=10:
        total += n * n
    println! "{total}"
    // Declarative: what -- an iterator chain.
    let total: u32 = (1..=10) <- map square <- sum$
    println! "{total}"
    // A pipe: the result flows into the next function.
    let line = ((1..=10) <- map square <- sum$) |> report
    println! "{line}"
    // A comprehension.
    let squares = list~ n * n for n in 1..=10u32
    let total: u32 = squares <- iter$ <- sum$
    println! "{total}"
```

```text
385
385
the sum of squares is 385
385
```

**In Harsh:** the same sum four ways — a loop, an iterator chain, a pipe
through a function, and a comprehension. The last three say what is computed;
only the first says how.

## 8.2 Generics as Type Classes

*Original: [Generics as Type Classes](https://rust-unofficial.github.io/patterns/functional/generics-type-classes.html)*

A generic type's parameters can select which methods exist: an `impl` for one
instantiation gives methods to that instantiation only — the way type classes
work in functional languages. The compiler then refuses, at compile time, a
method on the wrong kind of value.

```
use std.marker.PhantomData

struct Http
struct Ftp

// One request type; its parameter selects what it can do.
struct Request<P>
    url: String
    protocol: PhantomData<P>

impl<P> Request<P>
    fn url (&self) -> &str:
        &self <- url

impl Request<Http>
    fn new_http (url: &str) -> Self:
        Self\ url = url <- to_string$, protocol = PhantomData
    fn post (&self) (body: &str) -> String:
        format! "POST {} with {body:?}" (self <- url$)

impl Request<Ftp>
    fn new_ftp (url: &str) -> Self:
        Self\ url = url <- to_string$, protocol = PhantomData
    fn list (&self) -> String:
        format! "LIST {}" (self <- url$)

fn main$:
    let h = Request.new_http "http://harsh-lang.com"
    let f = Request.new_ftp "ftp://example.com"
    println! "{}" (h <- post "hi")
    println! "{}" (f <- list$)
    // `h <- list$` would not compile: `list` exists for FTP requests only.
```

```text
POST http://harsh-lang.com with "hi"
LIST ftp://example.com
```

**In Harsh:** the two `impl` blocks, for `Request<Http>` and `Request<Ftp>`,
read as two short lists of what each protocol allows.

## 8.3 Functional Optics

*Original: [Functional Optics](https://rust-unofficial.github.io/patterns/functional/optics.html)*

*Optics* are the functional vocabulary for conversions between types. An
**iso** converts both ways and loses nothing; a **poly iso** is an iso that
works for every type parameter; a **prism** converts one way always and the
other way only sometimes — a parse that may fail, and the rendering that
always succeeds. The original uses them to explain the design of Serde's API,
whose deserializer drives a visitor that may fail — a prism in all but name.

```
use std.collections.VecDeque

// An iso: two conversions that undo each other.
#[derive Debug Clone Copy PartialEq]
struct Celsius f64
#[derive Debug Clone Copy PartialEq]
struct Fahrenheit f64

fn to_f (c: Celsius) -> Fahrenheit:
    Fahrenheit (c <- 0 * 9.0 / 5.0 + 32.0)
fn to_c (f: Fahrenheit) -> Celsius:
    Celsius ((f <- 0 - 32.0) * 5.0 / 9.0)

// A poly iso: the same, for every element type.
fn to_deque<T> (v: Vec<T>) -> VecDeque<T>:
    VecDeque.from v
fn to_vec<T> (d: VecDeque<T>) -> Vec<T>:
    Vec.from d

// A prism: one way may fail, the other always succeeds.
fn preview (s: &str) -> Option<u16>:
    s <- parse$ <- ok$
fn review (port: u16) -> String:
    port <- to_string$

fn main$:
    let boiling = Celsius 100.0
    println! "{:?} and back: {}" (to_f boiling) (to_c (to_f boiling) == boiling)
    println! "{:?}" (to_vec (to_deque (vec! 'a' 'b')))
    println! "{:?} {:?}" (preview "8080") (preview "http")
    println! "{}" (preview (&review 443) == Some 443)
```

```text
Fahrenheit(212.0) and back: true
['a', 'b']
Some(8080) None
true
```

**In Harsh:** each optic is a pair of plain functions; the round trips at the
end are the laws they obey.


# Additional Resources

*Original: [Additional Resources](https://rust-unofficial.github.io/patterns/additional_resources/index.html)*

The original closes with material behind the patterns rather than patterns
themselves; its main page is the design principles.

## 9.1 Design principles

*Original: [Design principles](https://rust-unofficial.github.io/patterns/additional_resources/design-principles.html)*

Principles older than Rust, which hold in Harsh unchanged; this companion
names them and leaves their discussion to the original:

- **SOLID** — a unit has one responsibility; open to extension, closed to
  modification; subtypes stand in for their types; small, specific
  interfaces; depend on abstractions (in Rust: traits), not concrete types.
- **Composite reuse, or composition over inheritance** — which Rust enforces,
  having no inheritance; see *Deref Polymorphism*.
- **DRY** — every piece of knowledge has one representation in the system.
- **KISS** — most systems work best kept simple.
- **Law of Demeter** — a unit talks to its immediate collaborators only.
- **Design by contract** — preconditions, postconditions and invariants,
  stated and checked.
- **Encapsulation** — the invariants a module keeps are its own business; see
  *Contain unsafety in small modules*.
- **Command–query separation** — a function either changes something or
  answers a question, not both.
- **Principle of least astonishment** — a component behaves as its users
  expect.
- **Linguistic modular units** — modules are units of the language itself, as
  Rust's `mod` and crates are.
- **Self-documentation** — the documentation lives with the code it
  describes, as `///` comments and their tested examples do.
- **Uniform access** — a service is used the same way whether it is stored or
  computed.
- **Single choice** — where a system must choose among alternatives, one
  module alone knows the full list — as one `enum` and its `match`es do.
- **Persistence closure** — storing a value stores everything it depends on.

Harsh's own principle is the one Rust keeps: say what you mean, and let the
compiler check it. The layout and the pipe only make what you mean easier to
read.
