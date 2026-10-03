# The Fast Track to Harsh

A quick reference to everything you write daily, spelled in Harsh — as of
Harsh 0.2.0. It does not teach: *The Harsh Programming Language* does, and
reading it is not optional. Keep this page beside it. Every example on this
page is transpiled by `check.sh`, so none of it can fall behind the language.

## Basics

| Task | Harsh |
| --- | --- |
| Bind a value | `let answer = 42` |
| Mutable binding | `let mut count = 0` |
| Type annotation | `let ratio: f64 = 0.5` |
| Several at once | `let (x, y) = (1, 2)` |
| Constant | `const MAX_USERS: u32 = 100` |
| Static | `static GREETING: &str = "hello"` |
| Shadowing | `let n = n * 2` |
| Print a line | `println! "Hello, {name}!"` |
| Print with arguments | `println! "{} + {} = {}" a b (a + b)` |
| Debug print | `println! "{:?}" v` |
| Print to stderr | `eprintln! "warning: {msg}"` |
| Comment | `let x = 1  // to the end of the line` |
| Call a function | `let s = add 2 3` |
| Call with no arguments | `let t = now$` |
| A non-atom argument | `let s = add (n * 2) (-1)` |
| A method | `let n = text <- len$` |
| A method with arguments | `let parts = line <- split ","` |
| A field | `let w = rect <- width` |
| A path | `let m = std.cmp.max 3 7` |

The rule behind every call: arguments follow the function, one atom each;
anything that is not an atom — an operator, a prefix `-`, `&` or `*` — goes
in parentheses. `f$` calls with no arguments.

## Operators and precedence

| Task | Harsh |
| --- | --- |
| Arithmetic | `let r = (a + b) * c / d % e` |
| Compare | `let ok = a >= b && c != d` |
| Bitwise | `let m = (a & b) \| (c ^ d) << 2` |
| Compound assignment | `total += price * qty` |
| Cast | `let f = n as f64` |
| Range, exclusive and inclusive | `let r = 0..10` · `let s = 1..=10` |
| Negative argument | `let a = abs_diff (-3) 4` |
| Reference argument | `let n = count_words (&text)` |
| Dereference argument | `let v = double (*p)` |
| Dereference then field | `let x = (*p) <- x` |
| Error propagation | `let n = s <- parse<i32>$?` |

Tightest first: atoms (`f$`, `v[i]`, `( … )`), application `f a b`, the
chain `<-`, `?`, the prefix operators, Rust's operators in Rust's order, the
pipes, assignment. Two traps: `f -1` is a subtraction, and `*p <- x`
dereferences the field `x`, not `p`.

## Strings and characters

| Task | Harsh |
| --- | --- |
| String literal (`&str`) | `let s = "hello"` |
| Owned `String` | `let mut s = String.from "hello"` |
| Empty `String` | `let mut s = String.new$` |
| Append | `s <- push_str " world"` |
| Append a character | `s <- push '!'` |
| Format into a `String` | `let msg = format! "{name} is {age}"` |
| Length in bytes | `let n = s <- len$` |
| Length in characters | `let n = s <- chars$ <- count$` |
| Slice | `let hello = &s[0..5]` |
| Contains | `let found = s <- contains "lo"` |
| Replace | `let t = s <- replace "l" "L"` |
| Upper case | `let up = s <- to_uppercase$` |
| Trim | `let t = line <- trim$` |
| Split into words | `let words: Vec<&str> = s <- split_whitespace$ <- collect$` |
| Parse a number | `let n: i32 = "42" <- parse$ <- unwrap$` |
| Number to `String` | `let s = 42 <- to_string$` |
| Character | `let c = 'h'` |
| Character code | `let code = 'A' as u32` |
| Loop over characters | `for c in s <- chars$: println! "{c}"` |
| Raw string | `let path = r"C:\temp\new"` |

## Numbers

| Task | Harsh |
| --- | --- |
| Integer types | `let a: i8 = -5` · `let b: u64 = 5` · `let i: usize = 0` |
| Floating point | `let x: f64 = 2.5` |
| Literal with type | `let n = 255u8` |
| Separators | `let big = 1_000_000` |
| Hex, binary | `let h = 0xff` · `let b = 0b1010` |
| Largest value | `let m = u32.MAX` |
| Checked arithmetic | `let r = a <- checked_add b` |
| Wrapping arithmetic | `let r = a <- wrapping_mul b` |
| Power | `let p = 2i32 <- pow 10` |
| Square root | `let r = 2.0f64 <- sqrt$` |
| Round | `let r = x <- round$` |
| Absolute value | `let a = n <- abs$` |
| Minimum, maximum | `let m = a <- min b` |
| Convert, may fail | `let small = u8.try_from big` |
| Convert, cannot fail | `let wide = i64.from n` |

## Collections

| Task | Harsh |
| --- | --- |
| Vector literal | `let v = vec! 1 2 3` |
| Empty vector | `let mut v: Vec<i32> = Vec.new$` |
| Push, pop | `v <- push 4` · `let last = v <- pop$` |
| Index | `let first = v[0]` |
| Index, safely | `let maybe = v <- get 10` |
| Length, emptiness | `let n = v <- len$` · `let e = v <- is_empty$` |
| Contains | `let has = v <- contains (&3)` |
| Sort | `v <- sort$` |
| Sort by a key | `v <- sort_by_key (\|p\| p <- age)` |
| Reverse | `v <- reverse$` |
| Slice | `let middle = &v[1..3]` |
| Array | `let a = [1, 2, 3]` |
| Array of zeros | `let z = [0; 8]` |
| Hash map | `let mut ages = HashMap.new$` |
| Insert | `ages <- insert "Ana" 31` |
| Look up | `let a = ages <- get "Ana"` |
| Count with `entry` | `*counts <- entry word <- or_insert 0 += 1` |
| Remove | `ages <- remove "Ana"` |
| Hash set | `let mut seen = HashSet.new$` |
| Insert into a set | `let new = seen <- insert 7` |
| Double-ended queue | `let mut q = VecDeque.new$` |

## Comprehensions

| Task | Harsh |
| --- | --- |
| A `Vec` by comprehension | `let squares = list~ n * n for n in 1..=10` |
| With a condition | `let evens = list~ n for n in 0..20 if n % 2 == 0` |
| Two loops | `let pairs = list~ (a, b) for a in 1..4 for b in a..4` |
| A set | `let lengths = set~ w <- len$ for w in &words` |
| A map, `key => value` | `let index = dict~ w => i for (i, w) in words <- iter$ <- enumerate$` |
| A lazy generator | `let squares = g~ n * n for n in 1..` |

## Control flow

| Task | Harsh |
| --- | --- |
| `if`, inline | `let sign = if n < 0: -1 else: 1` |
| `if` as a statement | `if n < 0: println! "negative"` |
| Several branches | `let size = if n < 10: "small" else if n < 100: "medium" else: "large"` |
| `for` over a range | `for i in 0..10: println! "{i}"` |
| `for` over a collection | `for x in &v: println! "{x}"` |
| With the index | `for (i, x) in v <- iter$ <- enumerate$: println! "{i}: {x}"` |
| `while` | `while n > 1: n /= 2` |
| Endless `loop` | `loop: if tick$: break` |
| `loop` with a value | `let found = loop: break 42` |
| Labelled loop | `'outer: for i in 0..10: for j in 0..10: if i * j > 20: break 'outer` |
| Skip an iteration | `for i in 0..10: if i % 2 == 0: continue` |
| `if let` | `if let Some x = maybe: println! "{x}"` |
| `while let` | `while let Some top = stack <- pop$: println! "{top}"` |
| `let … else` | `let Some n = first else: return` |
| Discard a block's last value | `for w in words: seen <- insert w` then a line `()` beneath |

Blocks over several lines are what you will write most:

```rust harsh
fn classify (n: i32) -> &'static str:
    if n < 0:
        "negative"
    else if n == 0:
        "zero"
    else:
        "positive"

fn count_new (words: &[&str]) -> usize:
    let mut seen = std.collections.HashSet.new$
    for w in words:
        seen <- insert w
        ()
    seen <- len$
```

## Pattern matching

| Task | Harsh |
| --- | --- |
| Two arms, inline | `let name = match n\ 0 => "zero", _ => "other"` |
| A pattern with a value | `let n = match maybe\ Some x => x, None => 0` |
| Destructure a tuple | `let (a, b) = pair` |
| Destructure a struct | `let Point\ x, y = p` |
| Part of a struct | `let Point\ x, .. = p` |
| Rename while destructuring | `let Point\ x: px, y: py = p` |
| `matches!` | `let digit = matches! c ('0'..='9')` |

More than two arms go one per line, no commas:

```rust harsh
fn describe (n: i32) -> String:
    match n\
        0 => String.from "zero"
        1 | 2 | 3 => String.from "a few"
        4..=9 => String.from "several"
        x if x < 0 => format! "negative {x}"
        _ => String.from "many"
```

## Functions and closures

| Task | Harsh |
| --- | --- |
| A function | `fn add (a: i32) (b: i32) -> i32: a + b` |
| No parameters | `fn answer$ -> i32: 42` |
| One bare parameter | `fn double n: i32 -> i32: n * 2` |
| No return value | `fn greet (name: &str): println! "Hello, {name}"` |
| Early return | `fn first_even (v: &[i32]) -> Option<i32>: v <- iter$ <- find (\|x\| *x % 2 == 0) <- copied$` |
| A closure | `let inc = \|x\| x + 1` |
| Annotated closure | `let add = \|a: i32, b: i32\| -> i32: a + b` |
| Capturing by value | `let show = move \|\| println! "{name}"` |
| Call a closure | `let three = inc 2` |
| Pipe forward | `let r = 5 \|> double \|> inc` |
| Pipe backward | `let r = inc <\| double <\| 5` |
| Several values into one call | `let s = 2 3 \|> add` |
| Partial application | `let add10 = 10 \|> add` |

A body of several statements goes beneath its header; so may any body, for
room:

```rust harsh
fn double n: i32 -> i32:
    n * 2

fn apply<F: Fn (i32) -> i32> (f: F) (x: i32) -> i32:
    f x

fn adder n: i32 -> impl Fn (i32) -> i32:
    move |x| x + n

fn print_all<T: std.fmt.Debug> (items: &[T]):
    for i in items:
        println! "{:?}" i
```

## Structs and enums

| Task | Harsh |
| --- | --- |
| Record struct, inline | `struct Size\ w: f64, h: f64` |
| Tuple struct | `struct Meters f64` |
| Unit struct | `struct Marker` |
| Build a struct | `let p = Point\ x = 1.0, y = 2.0` |
| Field shorthand | `let p = Point\ x, y` |
| Struct update | `let q = Point\ x = 5.0, ..p` |
| Tuple struct value | `let d = Meters 3.5` |
| Read a tuple field | `let m = d.0` |
| Enum, inline | `enum Dir\ North, South, East, West` |
| Enum value | `let d = Dir.North` |
| Derive traits | `#[derive Debug Clone PartialEq]` above the type |

Declarations take no mark; their fields or variants go beneath, and a literal
with more than two fields goes one field per line:

```rust harsh
#[derive Debug Clone]
struct Point
    x: f64
    y: f64

enum Shape
    Circle f64
    Rect
        w: f64
        h: f64
    Empty

fn origin_config$ -> Config:
    Config\
        name = String.from "origin"
        verbose = true
        retries = 3

struct Config
    name: String
    verbose: bool
    retries: u32
```

## Methods and traits

```rust harsh
use std.fmt

struct Circle
    r: f64

impl Circle
    fn new r: f64 -> Self:
        Self\ r
    fn area (&self) -> f64:
        3.14159 * self <- r * self <- r
    fn grow (&mut self) (by: f64):
        self <- r += by

trait Describe
    fn describe (&self) -> String
    fn shout (&self) -> String:
        self <- describe$ <- to_uppercase$

impl Describe for Circle
    fn describe (&self) -> String:
        format! "a circle of radius {}" (self <- r)

impl fmt.Display for Circle
    fn fmt (&self) (f: &mut fmt.Formatter) -> fmt.Result:
        write! f "Circle({})" (self <- r)
```

| Task | Harsh |
| --- | --- |
| Call an associated function | `let c = Circle.new 2.0` |
| Call a method | `let a = c <- area$` |
| Call a mutating method | `c <- grow 1.0` |
| Trait object | `let items: Vec<Box<dyn Describe>> = vec! (Box.new c)` |
| `impl Trait` argument | `fn show (item: &impl Describe): println! "{}" (item <- describe$)` |
| Lifetime | `fn longest<'a> (x: &'a str) (y: &'a str) -> &'a str: if x <- len$ > y <- len$: x else: y` |

## Ownership and borrowing

| Task | Harsh |
| --- | --- |
| Move | `let b = a` |
| Clone | `let b = a <- clone$` |
| Borrow | `let r = &s` |
| Borrow mutably | `let m = &mut s` |
| Pass a borrow | `let n = length (&s)` |
| Pass a mutable borrow | `change (&mut s)` |
| Drop early | `drop guard` |
| Box | `let b = Box.new 5` |
| Shared ownership | `let a = Rc.new 5` · `let b = Rc.clone (&a)` |
| Interior mutability | `let c = RefCell.new 0` · `*c <- borrow_mut$ += 1` |
| Thread-safe sharing | `let n = Arc.new (Mutex.new 0)` |

## Errors

| Task | Harsh |
| --- | --- |
| `Option` | `let found: Option<i32> = Some 3` |
| Default when absent | `let n = found <- unwrap_or 0` |
| `Result` | `let r: Result<i32, String> = Ok 1` |
| Propagate with `?` | `let n = text <- parse<i32>$?` |
| Convert `Option` to `Result` | `let v = found <- ok_or "missing"?` |
| Transform the value | `let doubled = found <- map (\|x\| x * 2)` |
| Panic | `panic! "unreachable state: {s}"` |
| Unwrap with a message | `let f = File.open path <- expect "cannot open"` |
| `main` that returns errors | `fn main$ -> Result<(), Box<dyn Error>>: run$` |

A custom error type:

```rust harsh
use std.fmt

#[derive Debug]
enum AppError
    NotFound String
    Invalid u32

impl fmt.Display for AppError
    fn fmt (&self) (f: &mut fmt.Formatter) -> fmt.Result:
        match self\
            AppError.NotFound name => write! f "{name} not found"
            AppError.Invalid code => write! f "invalid code {code}"

impl std.error.Error for AppError {}
```

## Iterators

| Task | Harsh |
| --- | --- |
| Map | `let doubled: Vec<i32> = v <- iter$ <- map (\|x\| x * 2) <- collect$` |
| Filter | `let big: Vec<&i32> = v <- iter$ <- filter (\|x\| **x > 10) <- collect$` |
| Sum | `let total: i32 = v <- iter$ <- sum$` |
| Count | `let n = v <- iter$ <- filter (\|x\| **x > 0) <- count$` |
| Any, all | `let any_neg = v <- iter$ <- any (\|x\| *x < 0)` |
| Find | `let first = v <- iter$ <- find (\|x\| **x > 3)` |
| Fold | `let product = v <- iter$ <- fold 1 (\|acc, x\| acc * x)` |
| Zip | `let pairs: Vec<_> = a <- iter$ <- zip (&b) <- collect$` |
| Take, skip | `let firsts: Vec<_> = v <- iter$ <- take 3 <- collect$` |
| Max | `let top = v <- iter$ <- max$` |
| Into a `String` | `let joined = words <- join ", "` |

A chain of three steps or more goes vertical:

```rust harsh
fn evens_squared (v: &[i32]) -> Vec<i32>:
    v
        <- iter$
        <- filter (|x| *x % 2 == 0)
        <- map (|x| x * x)
        <- collect$
```

## Modules and imports

| Task | Harsh |
| --- | --- |
| Import an item | `use std.collections.HashMap` |
| Import several | `use std.io.(self, Write)` |
| Import with a new name | `use std.fmt.Result as FmtResult` |
| Import everything | `use std.collections.*` |
| Public item | `pub fn area (w: f64) (h: f64) -> f64: w * h` |
| From the crate root | `use crate.shapes.Circle` |
| From the parent module | `use super.helper` |
| A module in its own file | `mod shapes` |

## Testing

```rust harsh
fn add (a: i32) (b: i32) -> i32:
    a + b

#[cfg test]
mod tests
    use super.*

    #[test]
    fn adds_two_numbers$:
        assert_eq! (add 2 3) 5

    #[test]
    #[should_panic]
    fn index_out_of_range$:
        let v = vec! 1 2 3
        v[99]
        ()
```

| Task | Harsh |
| --- | --- |
| Assert | `assert! (x > 0)` |
| Assert equal | `assert_eq! (add 2 2) 4` |
| Assert with a message | `assert! ok "expected success, got {code}"` |
| Run the tests | `hrs test` (a command, not Harsh) |

## Concurrency and async

| Task | Harsh |
| --- | --- |
| Spawn a thread | `let h = thread.spawn (\|\| work$)` |
| Wait for it | `let result = h <- join$ <- unwrap$` |
| A channel | `let (tx, rx) = mpsc.channel$` |
| Send, receive | `tx <- send 42 <- unwrap$` · `let n = rx <- recv$ <- unwrap$` |
| Shared counter | `*counter <- lock$ <- unwrap$ += 1` |
| Async function | `async fn fetch (url: &str) -> String: download url <- await` |
| Await | `let page = fetch "https://example.com" <- await` |

A thread whose body spans lines, inside a loop; the `()` after the body says
the loop body is worth nothing:

```rust harsh
fn spawn_all$:
    let (tx, rx) = std.sync.mpsc.channel$
    for id in 0..3:
        let tx = tx <- clone$
        std.thread.spawn move ||:
            tx <- send (id * id) <- unwrap$
        ()
    drop tx
    for n in rx: println! "{n}"
```

## Linear algebra

Add `hrs_std` to the project once: `hrs add hrs_std`.

| Task | Harsh |
| --- | --- |
| Matrix | `let a = m~ [1.0 2.0; 3.0 4.0]` |
| Vector | `let b = v~ [5.0, 6.0]` |
| Product | `let c = &a * &a` |
| Element | `let e = a[(0, 1)]` |
| Determinant | `let d = a <- det$` |
| Inverse, or a panic | `let i = a <- inv$` |
| Solve `a x = b`, or a panic | `let x = a <- solve (&b)` |
| Solve, the caller decides | `let x = a <- try_solve (&b)?` |
| Norm | `let n = b <- norm$` |

`solve` and `inv` stop the program on a singular matrix, as Julia does;
`try_solve` and `try_inv` return `Err (LinAlgError.Singular)` instead.

## Macros

| Task | Harsh |
| --- | --- |
| Call a Rust macro | `let v = vec! 1 2 3` |
| A Rust macro with a block | `assert! (v <- is_empty$ == false)` |
| Call a Harsh macro | `let squares = list~ n * n for n in 1..5` |
| A macro by its path | `let s = hrs_std.list~ n for n in 0..3` |

Harsh's own macros, written `~`, are expanded by `hrs` and are gone from the
Rust. A macro's own language — an `rsx!` tree, `view!` markup, `select!` arms
— goes in braces as Rust, with Harsh in holes `@: … :@`. The Book's chapter
23 teaches writing both kinds.

## The `hrs` tool

| Command | What it does |
| --- | --- |
| `hrs new hello` | a project laid out for Harsh |
| `hrs build` | transpile, then `cargo build`; errors point at `.hrs` lines |
| `hrs run` | build and run |
| `hrs test` | build and run the tests |
| `hrs check` · `hrs lint` | `cargo check`, `cargo clippy`, mapped to Harsh |
| `hrs watch` | rebuild on every save |
| `hrs fmt` · `hrs fmt --check` | lay out the files by Harsh's rules |
| `hrs add hrs_std` | add Harsh's crates (Rust crates: `cargo add`) |
| `hrs in.hrs -o out.rs` | transpile one file, anywhere |
| `hrs export` | the project as a plain Rust crate |
| `hrs-from file.rs` | convert Rust to Harsh |
| Editors | `hrs-lsp`: Enter and Tab by layout, format on save, hover, definitions, completion |
| Jupyter | the `harsh-kernel`: a cell's last value is shown; a last line `()` hides it |

## Three ways to use Harsh

| | Published to Harsh's registry | Published to crates.io |
| --- | --- | --- |
| **Harsh project** (`hrs new`) | **#1:** a Harsh crate | **#2:** a Rust crate written in Harsh |
| **Rust project** (`cargo new`) | (empty) | **#3:** Harsh files in a Rust crate |

- **#1:** `hrs new mylib`, `hrs build`, `hrs test`; shared by path today,
  Harsh macros included. Harsh's registry is still to come.
- **#2:** `hrs new`, `hrs build`, then `hrs export` and `cargo publish`.
  Exported Harsh macros do not survive the export.
- **#3:** `cargo new`; each `.hrs` file beside the `.rs` it produces, with
  `hrs src/parser.hrs -o src/parser.rs`; commit the `.rs`, then
  `cargo publish`.

## Layout style

`hrs fmt` applies the countable rules; the rest is judgment:

- A chain of three steps or more goes vertical, every `<-` under the first.
- A literal of three fields or more goes one field per line, no commas.
- An `if` with three branches or more goes on several lines.
- A literal inside a literal on one line is parenthesised:
  `Dog\ inner = (Animal\ name = n), good = true`.
- A `match` with more than two arms goes one arm per line.
- A call made long by a long argument goes vertical, one argument per line.
- No semicolons: a block that discards its last value ends with a line `()`.

## Naming

As in Rust: `snake_case` for functions, variables and modules;
`CamelCase` for types and traits; `SCREAMING_SNAKE_CASE` for constants and
statics. Harsh files end in `.hrs`.

## Where to read more

- *The Harsh Programming Language* — the Book; read it all.
- *Harsh by Example* — one runnable page per feature.
- *Harsh Design Patterns* and *The Harshonomicon* — the companions.
- The language guide — every rule, with its Rust beside it.
